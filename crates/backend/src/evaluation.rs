use std::cmp::Ordering;

use interface::{BinaryOp, DataType, Expr, ExprKind, Literal, UnaryOp, Value};

use crate::{database::Table, error::BackendError};

#[derive(Clone, Copy)]
enum ExpressionType {
    Integer,
    Float,
    String,
    Boolean,
    Null,
}

impl ExpressionType {
    const fn name(self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Float => "FLOAT",
            Self::String => "STRING",
            Self::Boolean => "BOOLEAN",
            Self::Null => "NULL",
        }
    }

    const fn is_numeric(self) -> bool {
        matches!(self, Self::Integer | Self::Float)
    }
}

impl From<DataType> for ExpressionType {
    fn from(data_type: DataType) -> Self {
        match data_type {
            DataType::Integer => Self::Integer,
            DataType::Float => Self::Float,
            DataType::String => Self::String,
            DataType::Boolean => Self::Boolean,
        }
    }
}

fn infer_expression_type(
    expression: &Expr,
    table: &Table,
    table_name: &str,
) -> Result<ExpressionType, BackendError> {
    match &expression.kind {
        ExprKind::Identifier(name) => table
            .columns
            .iter()
            .find(|column| column.name.eq_ignore_ascii_case(name))
            .map(|column| ExpressionType::from(column.data_type))
            .ok_or_else(|| BackendError::ColumnNotFound {
                table: table_name.to_string(),
                column: name.clone(),
            }),
        ExprKind::Literal(literal) => Ok(match literal {
            Literal::Integer(_) => ExpressionType::Integer,
            Literal::Float(_) => ExpressionType::Float,
            Literal::String(_) => ExpressionType::String,
            Literal::Boolean(_) => ExpressionType::Boolean,
            Literal::Null => ExpressionType::Null,
        }),
        ExprKind::Unary { op, expr } => {
            let operand = infer_expression_type(expr, table, table_name)?;
            match (op, operand) {
                (_, ExpressionType::Null) => Ok(ExpressionType::Null),
                (UnaryOp::Plus | UnaryOp::Minus, operand) if operand.is_numeric() => Ok(operand),
                (UnaryOp::Not, ExpressionType::Boolean) => Ok(ExpressionType::Boolean),
                _ => Err(BackendError::InvalidExpression(format!(
                    "operator {op:?} cannot be applied to {}",
                    operand.name()
                ))),
            }
        }
        ExprKind::Binary { left, op, right } => {
            let left = infer_expression_type(left, table, table_name)?;
            let right = infer_expression_type(right, table, table_name)?;
            infer_binary_type(left, *op, right)
        }
    }
}

fn infer_binary_type(
    left: ExpressionType,
    operator: BinaryOp,
    right: ExpressionType,
) -> Result<ExpressionType, BackendError> {
    match operator {
        BinaryOp::And | BinaryOp::Or => {
            if matches!(left, ExpressionType::Boolean | ExpressionType::Null)
                && matches!(right, ExpressionType::Boolean | ExpressionType::Null)
            {
                Ok(ExpressionType::Boolean)
            } else {
                Err(BackendError::InvalidExpression(format!(
                    "logical operators require BOOLEAN values, found {} and {}",
                    left.name(),
                    right.name()
                )))
            }
        }
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Modulo => {
            if [left, right]
                .into_iter()
                .all(|value| value.is_numeric() || matches!(value, ExpressionType::Null))
            {
                if matches!(left, ExpressionType::Float) || matches!(right, ExpressionType::Float) {
                    Ok(ExpressionType::Float)
                } else if matches!(left, ExpressionType::Null)
                    && matches!(right, ExpressionType::Null)
                {
                    Ok(ExpressionType::Null)
                } else {
                    Ok(ExpressionType::Integer)
                }
            } else {
                Err(BackendError::InvalidExpression(format!(
                    "arithmetic requires numeric values, found {} and {}",
                    left.name(),
                    right.name()
                )))
            }
        }
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::LessThan
        | BinaryOp::LessThanOrEqual
        | BinaryOp::GreaterThan
        | BinaryOp::GreaterThanOrEqual => {
            let compatible = matches!(left, ExpressionType::Null)
                || matches!(right, ExpressionType::Null)
                || (left.is_numeric() && right.is_numeric())
                || matches!(
                    (left, right),
                    (ExpressionType::String, ExpressionType::String)
                )
                || (matches!(operator, BinaryOp::Equal | BinaryOp::NotEqual)
                    && matches!(
                        (left, right),
                        (ExpressionType::Boolean, ExpressionType::Boolean)
                    ));
            if compatible {
                Ok(ExpressionType::Boolean)
            } else {
                Err(BackendError::InvalidExpression(format!(
                    "cannot compare {} with {}",
                    left.name(),
                    right.name()
                )))
            }
        }
    }
}

pub(crate) fn validate_expression(
    expression: &Expr,
    table: &Table,
    table_name: &str,
) -> Result<(), BackendError> {
    infer_expression_type(expression, table, table_name).map(|_| ())
}

pub(crate) fn validate_where_expression(
    expression: &Expr,
    table: &Table,
    table_name: &str,
) -> Result<(), BackendError> {
    let expression_type = infer_expression_type(expression, table, table_name)?;
    if matches!(
        expression_type,
        ExpressionType::Boolean | ExpressionType::Null
    ) {
        Ok(())
    } else {
        Err(BackendError::InvalidExpression(format!(
            "WHERE expression must produce BOOLEAN, found {}",
            expression_type.name()
        )))
    }
}

pub(crate) fn eval_on_row(
    expression: &Expr,
    row: &[Value],
    table: &Table,
) -> Result<Value, BackendError> {
    eval_expr(expression, &|name| {
        let index = table
            .columns
            .iter()
            .position(|column| column.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| BackendError::InvalidExpression(format!("unknown column '{name}'")))?;
        row.get(index)
            .cloned()
            .ok_or_else(|| BackendError::InvalidSchema(format!("row is missing column '{name}'")))
    })
}

pub(crate) fn eval_const(expression: &Expr) -> Result<Value, BackendError> {
    eval_expr(expression, &|name| {
        Err(BackendError::InvalidExpression(format!(
            "INSERT value may not reference column '{name}'"
        )))
    })
}

fn eval_expr(
    expression: &Expr,
    lookup: &impl Fn(&str) -> Result<Value, BackendError>,
) -> Result<Value, BackendError> {
    match &expression.kind {
        ExprKind::Literal(literal) => Ok(match literal {
            Literal::Integer(value) => Value::Integer(*value),
            Literal::Float(value) => Value::Float(*value),
            Literal::String(value) => Value::String(value.clone()),
            Literal::Boolean(value) => Value::Boolean(*value),
            Literal::Null => Value::Null,
        }),
        ExprKind::Identifier(name) => lookup(name),
        ExprKind::Unary { op, expr } => eval_unary(*op, eval_expr(expr, lookup)?),
        ExprKind::Binary { left, op, right } => {
            let left = eval_expr(left, lookup)?;
            let right = eval_expr(right, lookup)?;
            eval_binary(left, *op, right)
        }
    }
}

fn eval_unary(operator: UnaryOp, value: Value) -> Result<Value, BackendError> {
    match (operator, value) {
        (_, Value::Null) => Ok(Value::Null),
        (UnaryOp::Plus, value @ (Value::Integer(_) | Value::Float(_))) => Ok(value),
        (UnaryOp::Minus, Value::Integer(value)) => value
            .checked_neg()
            .map(Value::Integer)
            .ok_or_else(|| BackendError::Execution("integer overflow in negation".into())),
        (UnaryOp::Minus, Value::Float(value)) => finite_float(-value),
        (UnaryOp::Not, Value::Boolean(value)) => Ok(Value::Boolean(!value)),
        (operator, value) => Err(BackendError::InvalidExpression(format!(
            "operator {operator:?} cannot be applied to {}",
            value.type_name()
        ))),
    }
}

fn eval_binary(left: Value, operator: BinaryOp, right: Value) -> Result<Value, BackendError> {
    match operator {
        BinaryOp::And | BinaryOp::Or => eval_logical(left, operator, right),
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::LessThan
        | BinaryOp::LessThanOrEqual
        | BinaryOp::GreaterThan
        | BinaryOp::GreaterThanOrEqual => eval_comparison(left, operator, right),
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Modulo => eval_arithmetic(left, operator, right),
    }
}

fn eval_logical(left: Value, operator: BinaryOp, right: Value) -> Result<Value, BackendError> {
    use Value::{Boolean, Null};
    match (left, operator, right) {
        (Boolean(false), BinaryOp::And, _) | (_, BinaryOp::And, Boolean(false)) => {
            Ok(Boolean(false))
        }
        (Boolean(true), BinaryOp::And, Boolean(true)) => Ok(Boolean(true)),
        (Boolean(true), BinaryOp::Or, _) | (_, BinaryOp::Or, Boolean(true)) => Ok(Boolean(true)),
        (Boolean(false), BinaryOp::Or, Boolean(false)) => Ok(Boolean(false)),
        (Null, BinaryOp::And | BinaryOp::Or, Boolean(_))
        | (Boolean(_), BinaryOp::And | BinaryOp::Or, Null)
        | (Null, BinaryOp::And | BinaryOp::Or, Null) => Ok(Null),
        (left, _, right) => Err(BackendError::InvalidExpression(format!(
            "logical operators require BOOLEAN values, found {} and {}",
            left.type_name(),
            right.type_name()
        ))),
    }
}

fn eval_comparison(left: Value, operator: BinaryOp, right: Value) -> Result<Value, BackendError> {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return Ok(Value::Null);
    }

    let ordering = match (&left, &right) {
        (Value::Integer(left), Value::Integer(right)) => Some(left.cmp(right)),
        (Value::Float(left), Value::Float(right)) => left.partial_cmp(right),
        (Value::Integer(left), Value::Float(right)) => (*left as f64).partial_cmp(right),
        (Value::Float(left), Value::Integer(right)) => left.partial_cmp(&(*right as f64)),
        (Value::String(left), Value::String(right)) => Some(left.cmp(right)),
        (Value::Boolean(left), Value::Boolean(right))
            if matches!(operator, BinaryOp::Equal | BinaryOp::NotEqual) =>
        {
            Some(left.cmp(right))
        }
        _ => {
            return Err(BackendError::InvalidExpression(format!(
                "cannot compare {} with {}",
                left.type_name(),
                right.type_name()
            )));
        }
    }
    .ok_or_else(|| BackendError::InvalidExpression("comparison is undefined".into()))?;

    let result = match operator {
        BinaryOp::Equal => ordering == Ordering::Equal,
        BinaryOp::NotEqual => ordering != Ordering::Equal,
        BinaryOp::LessThan => ordering == Ordering::Less,
        BinaryOp::LessThanOrEqual => ordering != Ordering::Greater,
        BinaryOp::GreaterThan => ordering == Ordering::Greater,
        BinaryOp::GreaterThanOrEqual => ordering != Ordering::Less,
        _ => unreachable!("caller only supplies comparison operators"),
    };
    Ok(Value::Boolean(result))
}

fn eval_arithmetic(left: Value, operator: BinaryOp, right: Value) -> Result<Value, BackendError> {
    if matches!(left, Value::Null) || matches!(right, Value::Null) {
        return Ok(Value::Null);
    }

    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => {
            let value = match operator {
                BinaryOp::Add => left.checked_add(right),
                BinaryOp::Subtract => left.checked_sub(right),
                BinaryOp::Multiply => left.checked_mul(right),
                BinaryOp::Divide if right == 0 => {
                    return Err(BackendError::Execution("division by zero".into()));
                }
                BinaryOp::Divide => left.checked_div(right),
                BinaryOp::Modulo if right == 0 => {
                    return Err(BackendError::Execution("division by zero".into()));
                }
                BinaryOp::Modulo => left.checked_rem(right),
                _ => unreachable!("caller only supplies arithmetic operators"),
            }
            .ok_or_else(|| BackendError::Execution("integer arithmetic overflow".into()))?;
            Ok(Value::Integer(value))
        }
        (Value::Integer(left), Value::Float(right)) => {
            eval_float_arithmetic(left as f64, operator, right)
        }
        (Value::Float(left), Value::Integer(right)) => {
            eval_float_arithmetic(left, operator, right as f64)
        }
        (Value::Float(left), Value::Float(right)) => eval_float_arithmetic(left, operator, right),
        (left, right) => Err(BackendError::InvalidExpression(format!(
            "arithmetic requires numeric values, found {} and {}",
            left.type_name(),
            right.type_name()
        ))),
    }
}

fn eval_float_arithmetic(left: f64, operator: BinaryOp, right: f64) -> Result<Value, BackendError> {
    if right == 0.0 && matches!(operator, BinaryOp::Divide | BinaryOp::Modulo) {
        return Err(BackendError::Execution("division by zero".into()));
    }
    let result = match operator {
        BinaryOp::Add => left + right,
        BinaryOp::Subtract => left - right,
        BinaryOp::Multiply => left * right,
        BinaryOp::Divide => left / right,
        BinaryOp::Modulo => left % right,
        _ => unreachable!("caller only supplies arithmetic operators"),
    };
    finite_float(result)
}

fn finite_float(value: f64) -> Result<Value, BackendError> {
    if value.is_finite() {
        Ok(Value::Float(value))
    } else {
        Err(BackendError::Execution(
            "floating-point operation produced a non-finite value".into(),
        ))
    }
}
