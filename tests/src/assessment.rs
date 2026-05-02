use assessment_engine::models::exercise::{Difficulty, Exercise, ExerciseType, TestCase};
use assessment_engine::AssessmentEngine;
use serde_json::json;

#[test]
fn test_multiple_choice_correct_answer() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_multiple_choice(
        "ch1",
        "What is 2+2?",
        vec!["3".to_string(), "4".to_string(), "5".to_string()],
        1,
        1.0,
    );

    let answer = json!({"selected_index": 1});
    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 1.0);
    assert!(result.is_correct);
    assert_eq!(result.feedback, "Correct!");
}

#[test]
fn test_multiple_choice_wrong_answer() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_multiple_choice(
        "ch1",
        "What is 2+2?",
        vec!["3".to_string(), "4".to_string(), "5".to_string()],
        1,
        1.0,
    );

    let answer = json!({"selected_index": 0});
    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
    assert!(result.feedback.contains("option 2"));
}

#[test]
fn test_multiple_choice_deterministic() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_multiple_choice(
        "ch1",
        "What is 2+2?",
        vec!["3".to_string(), "4".to_string(), "5".to_string()],
        1,
        1.0,
    );

    let answer = json!({"selected_index": 1});

    // Run 100 times, should get same result
    for _ in 0..100 {
        let result = engine.evaluate(&exercise, &answer).unwrap();
        assert_eq!(result.score, 1.0);
        assert!(result.is_correct);
    }
}

#[test]
fn test_short_answer_good_response() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_short_answer(
        "ch1",
        "Explain what Rust is",
        "Rust is a systems programming language",
        vec![
            "systems programming".to_string(),
            "memory safety".to_string(),
            "performance".to_string(),
        ],
        3.0,
    );

    let answer = json!({
        "answer": "Rust is a systems programming language focused on memory safety and performance"
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert!(result.score > 0.0);
    assert!(result.is_correct);
}

#[test]
fn test_short_answer_empty() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_short_answer(
        "ch1",
        "Explain what Rust is",
        "Rust is a systems programming language",
        vec!["systems programming".to_string()],
        1.0,
    );

    let answer = json!({"answer": ""});
    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
}

#[test]
fn test_coding_valid_submission() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_coding(
        "ch1",
        "Write a function to add two numbers",
        "python",
        vec![
            TestCase {
                input: "2, 3".to_string(),
                expected_output: "5".to_string(),
            },
            TestCase {
                input: "-1, 1".to_string(),
                expected_output: "0".to_string(),
            },
        ],
        2.0,
    );

    let answer = json!({
        "code": "def add(a, b):\n    return a + b"
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 2.0);
    assert!(result.is_correct);
}

#[test]
fn test_coding_empty_submission() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_coding(
        "ch1",
        "Write a function to add two numbers",
        "python",
        vec![TestCase {
            input: "2, 3".to_string(),
            expected_output: "5".to_string(),
        }],
        1.0,
    );

    let answer = json!({"code": ""});
    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
}

#[test]
fn test_reflection_meets_length() {
    let engine = AssessmentEngine::new();
    let dimensions = vec![assessment_engine::models::exercise::RubricDimension {
        name: "understanding".to_string(),
        description: "Demonstrates understanding of the concept".to_string(),
        max_score: 2.0,
    }];

    let exercise = Exercise {
        id: uuid::Uuid::new_v4(),
        chapter_id: "ch1".to_string(),
        question: "Reflect on what you learned".to_string(),
        exercise_type: ExerciseType::Reflection {
            prompt: "Write about what you learned".to_string(),
            min_length: 50,
            rubric_dimensions: dimensions.clone(),
        },
        difficulty: Difficulty::Medium,
        rubric: None,
        max_score: 2.0,
        hints: Vec::new(),
        explanation: None,
    };

    let answer = json!({
        "reflection": "This chapter demonstrates that Rust is a systems programming language focusing on memory safety and performance. I gained a deeper understanding of the concept of ownership which is central to Rust's design."
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert!(result.score > 0.0);
    assert!(result.is_correct);
}

#[test]
fn test_reflection_too_short() {
    let engine = AssessmentEngine::new();
    let dimensions = vec![];
    let exercise = Exercise {
        id: uuid::Uuid::new_v4(),
        chapter_id: "ch1".to_string(),
        question: "Reflect on what you learned".to_string(),
        exercise_type: ExerciseType::Reflection {
            prompt: "Write about what you learned".to_string(),
            min_length: 100,
            rubric_dimensions: dimensions.clone(),
        },
        difficulty: Difficulty::Medium,
        rubric: None,
        max_score: 2.0,
        hints: Vec::new(),
        explanation: None,
    };

    let answer = json!({
        "reflection": "Short reflection"
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
    assert!(result.feedback.contains("too short"));
}

#[test]
fn test_invalid_answer_format() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_multiple_choice(
        "ch1",
        "What is 2+2?",
        vec!["3".to_string(), "4".to_string(), "5".to_string()],
        1,
        1.0,
    );

    let answer = json!({"wrong_field": 1});
    let result = engine.evaluate(&exercise, &answer);

    assert!(result.is_err());
}

// ── Edge case tests ──

#[test]
fn test_multiple_choice_out_of_bounds_index() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_multiple_choice(
        "ch1",
        "What is 2+2?",
        vec!["3".to_string(), "4".to_string(), "5".to_string()],
        1,
        1.0,
    );

    // Index 99 is out of bounds
    let answer = json!({"selected_index": 99});
    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
}

#[test]
fn test_multiple_choice_first_option_correct() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_multiple_choice(
        "ch1",
        "Pick the first",
        vec!["Correct".to_string(), "Wrong".to_string()],
        0,
        1.0,
    );

    let answer = json!({"selected_index": 0});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert!(result.is_correct);
}

#[test]
fn test_short_answer_all_key_points_missed() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_short_answer(
        "ch1",
        "Explain quantum computing",
        "Uses qubits and superposition",
        vec![
            "qubits".to_string(),
            "superposition".to_string(),
            "entanglement".to_string(),
        ],
        3.0,
    );

    let answer = json!({"answer": "It is a type of cooking that uses quantum ovens"});
    let result = engine.evaluate(&exercise, &answer).unwrap();

    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
}

#[test]
fn test_short_answer_unicode_content() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_short_answer(
        "ch1",
        "What is Rust?",
        "Rust is safe",
        vec!["sécurité".to_string(), "性能".to_string()],
        2.0,
    );

    // Answer contains one of the Unicode key points
    let answer = json!({"answer": "Rust offre une grande sécurité mémoire"});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert!(result.score > 0.0);
}

#[test]
fn test_coding_no_test_cases_gives_full_score() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_coding("ch1", "Write a function", "python", vec![], 2.0);

    let answer = json!({"code": "def foo():\n    pass"});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert_eq!(result.score, 2.0);
}

#[test]
fn test_reflection_exact_min_length_boundary() {
    let engine = AssessmentEngine::new();
    let dimensions = vec![assessment_engine::models::exercise::RubricDimension {
        name: "clarity".to_string(),
        description: "clear writing clarity".to_string(),
        max_score: 2.0,
    }];

    let min_len = 20;
    let exact_text = "x".repeat(min_len); // exactly at boundary

    let exercise = Exercise {
        id: uuid::Uuid::new_v4(),
        chapter_id: "ch1".to_string(),
        question: "Reflect".to_string(),
        exercise_type: ExerciseType::Reflection {
            prompt: "Write".to_string(),
            min_length: min_len,
            rubric_dimensions: dimensions.clone(),
        },
        difficulty: Difficulty::Medium,
        rubric: None,
        max_score: 2.0,
        hints: Vec::new(),
        explanation: None,
    };

    let answer = json!({"reflection": exact_text});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    // Should NOT fail with "too short" — it's exactly at the boundary
    assert!(!result.feedback.contains("too short"));
}

#[test]
fn test_reflection_barely_over_min_length() {
    let engine = AssessmentEngine::new();
    let dimensions = vec![];

    let min_len = 50;
    let text = "x".repeat(min_len + 1); // one over boundary

    let exercise = Exercise {
        id: uuid::Uuid::new_v4(),
        chapter_id: "ch1".to_string(),
        question: "Reflect".to_string(),
        exercise_type: ExerciseType::Reflection {
            prompt: "Write".to_string(),
            min_length: min_len,
            rubric_dimensions: dimensions.clone(),
        },
        difficulty: Difficulty::Medium,
        rubric: None,
        max_score: 2.0,
        hints: Vec::new(),
        explanation: None,
    };

    let answer = json!({"reflection": text});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    // No rubric dimensions + meets min length → full score
    assert_eq!(result.score, 2.0);
    assert!(result.is_correct);
}

#[test]
fn test_scorer_failing_grade() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(
        uuid::Uuid::new_v4(),
        json!({}),
        30.0,
        100.0,
        "Needs work".to_string(),
    );
    assert_eq!(Scorer::calculate_percentage(&eval), 30.0);
    assert_eq!(Scorer::grade_letter(&eval), 'F');
    assert!(!Scorer::is_passing(&eval, 0.7));
}

#[test]
fn test_scorer_perfect_grade() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(
        uuid::Uuid::new_v4(),
        json!({}),
        95.0,
        100.0,
        "Perfect".to_string(),
    );
    assert_eq!(Scorer::calculate_percentage(&eval), 95.0);
    assert_eq!(Scorer::grade_letter(&eval), 'A');
    assert!(Scorer::is_passing(&eval, 0.7));
}

#[test]
fn test_scorer_boundary_a() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(uuid::Uuid::new_v4(), json!({}), 90.0, 100.0, "".to_string());
    assert_eq!(Scorer::grade_letter(&eval), 'A');
}

#[test]
fn test_scorer_boundary_b() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(uuid::Uuid::new_v4(), json!({}), 80.0, 100.0, "".to_string());
    assert_eq!(Scorer::grade_letter(&eval), 'B');
}

#[test]
fn test_scorer_boundary_c() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(uuid::Uuid::new_v4(), json!({}), 70.0, 100.0, "".to_string());
    assert_eq!(Scorer::grade_letter(&eval), 'C');
}

#[test]
fn test_scorer_boundary_d() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(uuid::Uuid::new_v4(), json!({}), 60.0, 100.0, "".to_string());
    assert_eq!(Scorer::grade_letter(&eval), 'D');
}

#[test]
fn test_scorer_zero_score() {
    use assessment_engine::evaluation::scorer::Scorer;
    use assessment_engine::models::evaluation::Evaluation;

    let eval = Evaluation::new(uuid::Uuid::new_v4(), json!({}), 0.0, 100.0, "".to_string());
    assert_eq!(Scorer::calculate_percentage(&eval), 0.0);
    assert_eq!(Scorer::grade_letter(&eval), 'F');
    assert!(!Scorer::is_passing(&eval, 0.7));
}

// ── Phase 2: Additional boundary tests ──

#[test]
fn test_multiple_choice_single_option() {
    let engine = AssessmentEngine::new();
    let exercise =
        Exercise::new_multiple_choice("ch1", "Is this true?", vec!["Yes".to_string()], 0, 1.0);

    let answer = json!({"selected_index": 0});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert_eq!(result.score, 1.0);
    assert!(result.is_correct);
}

#[test]
fn test_multiple_choice_many_options() {
    let engine = AssessmentEngine::new();
    let options: Vec<String> = (0..10).map(|i| format!("Option {}", i)).collect();
    let exercise = Exercise::new_multiple_choice("ch1", "Pick the correct one", options, 7, 1.0);

    let answer = json!({"selected_index": 7});
    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert_eq!(result.score, 1.0);
    assert!(result.is_correct);
}

#[test]
fn test_short_answer_all_key_points_matched() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_short_answer(
        "ch1",
        "What are the key features of Rust?",
        "Rust has memory safety, zero-cost abstractions, and concurrency",
        vec![
            "memory safety".to_string(),
            "zero-cost abstractions".to_string(),
            "concurrency".to_string(),
        ],
        3.0,
    );

    let answer = json!({
        "answer": "Rust has memory safety, zero-cost abstractions, and concurrency support"
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert!(result.score > 0.0);
    assert!(result.is_correct);
}

#[test]
fn test_coding_with_multiple_test_cases() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise::new_coding(
        "ch1",
        "Write a function to check if a number is even",
        "python",
        vec![
            TestCase {
                input: "2".to_string(),
                expected_output: "True".to_string(),
            },
            TestCase {
                input: "3".to_string(),
                expected_output: "False".to_string(),
            },
            TestCase {
                input: "0".to_string(),
                expected_output: "True".to_string(),
            },
        ],
        3.0,
    );

    let answer = json!({
        "code": "def is_even(n):\n    return n % 2 == 0"
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert_eq!(result.score, 3.0);
    assert!(result.is_correct);
}

#[test]
fn test_reflection_with_rubric_dimensions() {
    let engine = AssessmentEngine::new();
    let dimensions = vec![
        assessment_engine::models::exercise::RubricDimension {
            name: "understanding".to_string(),
            description: "Demonstrates understanding".to_string(),
            max_score: 2.0,
        },
        assessment_engine::models::exercise::RubricDimension {
            name: "depth".to_string(),
            description: "Provides depth analysis".to_string(),
            max_score: 2.0,
        },
    ];

    let exercise = Exercise {
        id: uuid::Uuid::new_v4(),
        chapter_id: "ch1".to_string(),
        question: "Reflect on what you learned".to_string(),
        exercise_type: ExerciseType::Reflection {
            prompt: "Write about what you learned".to_string(),
            min_length: 50,
            rubric_dimensions: dimensions,
        },
        difficulty: Difficulty::Medium,
        rubric: None,
        max_score: 4.0,
        hints: Vec::new(),
        explanation: None,
    };

    let answer = json!({
        "reflection": "This chapter demonstrates that Rust is a systems programming language focusing on memory safety and performance. I gained a deeper understanding of the concept of ownership which is central to Rust's design. The depth of analysis shows how these concepts work together."
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert!(result.score > 0.0);
    assert!(result.is_correct);
}

#[test]
fn test_coding_with_starter_code() {
    let engine = AssessmentEngine::new();
    let exercise = Exercise {
        id: uuid::Uuid::new_v4(),
        chapter_id: "ch1".to_string(),
        question: "Complete the function".to_string(),
        exercise_type: ExerciseType::Coding {
            language: "python".to_string(),
            starter_code: Some("def add(a, b):\n    # TODO: implement\n    pass".to_string()),
            test_cases: vec![TestCase {
                input: "2, 3".to_string(),
                expected_output: "5".to_string(),
            }],
        },
        difficulty: Difficulty::Easy,
        rubric: None,
        max_score: 1.0,
        hints: Vec::new(),
        explanation: None,
    };

    let answer = json!({
        "code": "def add(a, b):\n    return a + b"
    });

    let result = engine.evaluate(&exercise, &answer).unwrap();
    assert_eq!(result.score, 1.0);
    assert!(result.is_correct);
}

#[test]
fn test_exercise_creation_helpers() {
    let mc = Exercise::new_multiple_choice(
        "ch1",
        "Question",
        vec!["A".to_string(), "B".to_string()],
        0,
        1.0,
    );
    assert_eq!(mc.max_score, 1.0);

    let sa = Exercise::new_short_answer("ch1", "Question", "Answer", vec!["key".to_string()], 2.0);
    assert_eq!(sa.max_score, 2.0);

    let coding = Exercise::new_coding("ch1", "Question", "python", vec![], 3.0);
    assert_eq!(coding.max_score, 3.0);
}
