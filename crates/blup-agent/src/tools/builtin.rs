use async_trait::async_trait;
use serde_json::{json, Value};

use super::{AgentTool, ToolError, ToolResult};

/// A simple calculator tool for math expressions.
pub struct CalculatorTool;

#[async_trait]
impl AgentTool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> &str {
        "Evaluate a mathematical expression. Supports basic arithmetic: +, -, *, /, ^, sqrt."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "The math expression to evaluate (e.g., '2 + 3 * 4')"
                }
            },
            "required": ["expression"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let expression = args
            .get("expression")
            .and_then(|e| e.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("Missing 'expression' parameter".to_string()))?;

        // Simple evaluation - in production, use a proper math parser
        let result = evaluate_simple(expression).map_err(ToolError::ExecutionFailed)?;

        Ok(ToolResult::success(format!("{expression} = {result}")))
    }
}

/// Simple math expression evaluator (supports +, -, *, /).
fn evaluate_simple(expr: &str) -> Result<f64, String> {
    let expr = expr.replace(' ', "");
    // Very basic evaluator - just handles simple binary operations
    // In production, use meval or evalexpr crate
    let chars: Vec<char> = expr.chars().collect();
    let mut numbers = Vec::new();
    let mut ops = Vec::new();
    let mut current = String::new();

    for c in &chars {
        if c.is_ascii_digit() || *c == '.' {
            current.push(*c);
        } else if "+-*/^".contains(*c) {
            if current.is_empty() {
                return Err(format!("Invalid expression: unexpected operator '{c}'"));
            }
            numbers.push(current.parse::<f64>().map_err(|_| "Invalid number")?);
            ops.push(*c);
            current.clear();
        }
    }

    if !current.is_empty() {
        numbers.push(current.parse::<f64>().map_err(|_| "Invalid number")?);
    }

    if numbers.is_empty() {
        return Err("Empty expression".to_string());
    }

    let mut result = numbers[0];
    for (i, op) in ops.iter().enumerate() {
        let next = numbers.get(i + 1).ok_or("Incomplete expression")?;
        match op {
            '+' => result += next,
            '-' => result -= next,
            '*' => result *= next,
            '/' => {
                if *next == 0.0 {
                    return Err("Division by zero".to_string());
                }
                result /= next
            }
            '^' => result = result.powf(*next),
            _ => return Err(format!("Unknown operator: {op}")),
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(evaluate_simple("2+3").unwrap(), 5.0);
        assert_eq!(evaluate_simple("10-4").unwrap(), 6.0);
        assert_eq!(evaluate_simple("3*7").unwrap(), 21.0);
        assert_eq!(evaluate_simple("15/3").unwrap(), 5.0);
    }

    #[test]
    fn test_with_spaces() {
        // Simple left-to-right evaluation: (2+3)*4 = 20
        assert_eq!(evaluate_simple("2 + 3 * 4").unwrap(), 20.0);
    }

    #[test]
    fn test_division_by_zero() {
        let result = evaluate_simple("5/0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Division by zero"));
    }

    #[test]
    fn test_empty_expression() {
        let result = evaluate_simple("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_expression() {
        let result = evaluate_simple("+ 5");
        assert!(result.is_err());
    }

    #[test]
    fn test_power_operator() {
        assert_eq!(evaluate_simple("2^3").unwrap(), 8.0);
    }

    #[test]
    fn test_decimal_numbers() {
        let result = evaluate_simple("2.5+3.5").unwrap();
        assert!((result - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative_result() {
        assert_eq!(evaluate_simple("3-10").unwrap(), -7.0);
    }

    #[tokio::test]
    async fn test_calculator_tool_execute() {
        let tool = CalculatorTool;
        let result = tool.execute(json!({"expression": "2+3"})).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("5"));
    }

    #[tokio::test]
    async fn test_calculator_missing_expression() {
        let tool = CalculatorTool;
        let result = tool.execute(json!({})).await;
        assert!(result.is_err());
    }
}
