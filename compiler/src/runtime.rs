#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAbi {
    pub print_utf8: &'static str,
    pub print_i64: &'static str,
    pub exit: &'static str,
}

impl RuntimeAbi {
    pub fn windows_x64() -> Self {
        Self {
            print_utf8: "tailang_rt_print_utf8",
            print_i64: "tailang_rt_print_i64",
            exit: "tailang_rt_exit",
        }
    }
}
