//! Testing utilities for quality assurance

use super::*;

pub struct QualityTester {
    test_suites: Vec<TestSuite>,
}

impl QualityTester {
    pub fn new() -> Self {
        Self {
            test_suites: Vec::new(),
        }
    }

    pub fn add_test_suite(&mut self, suite: TestSuite) {
        self.test_suites.push(suite);
    }

    pub fn run_all_tests(&self) -> TestResults {
        let mut passed = 0;
        let mut failed = 0;

        for suite in &self.test_suites {
            for test in &suite.tests {
                if (test.test_fn)() {
                    passed += 1;
                } else {
                    failed += 1;
                }
            }
        }

        TestResults {
            total: passed + failed,
            passed,
            failed,
            success_rate: if passed + failed > 0 {
                (passed as f64 / (passed + failed) as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

impl Default for QualityTester {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestSuite {
    pub name: String,
    pub tests: Vec<Test>,
}

pub struct Test {
    pub name: String,
    pub test_fn: Box<dyn Fn() -> bool + Send + Sync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub success_rate: f64,
}
