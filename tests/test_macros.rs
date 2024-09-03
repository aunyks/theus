use theus::c_compatible;

#[test]
fn test_basic_struct() {
    struct TestStruct {
        value: i32,
    }

    #[c_compatible]
    impl TestStruct {
        pub fn create(value: i32) -> Self {
            TestStruct { value }
        }

        pub fn get_value(&mut self) -> i32 {
            self.value
        }

        pub fn destroy(self) {}
    }
}

#[test]
fn test_trait_impl() {
    struct TestStruct {
        value: i32,
    }

    trait TestTrait {
        fn trait_method(&mut self, x: i32) -> i32;
    }

    #[c_compatible]
    impl TestTrait for TestStruct {
        fn trait_method(&mut self, x: i32) -> i32 {
            self.value + x
        }
    }
}
