//! Vela Protocol Buffers
//!
//! 这个 crate 提供了有条件编译的 protobuf 定义。
//! 使用不同的 feature 来启用不同的协议模块。

mod io;

pub use io::{FrameError, Framed};

// 包含生成的 protobuf 代码
#[cfg(feature = "common")]
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/vela.common.rs"));

    // OK = 0; // 成功 HTTP 200
    // CANCELLED = 1; // 操作被取消 HTTP 499
    // UNKNOWN = 2; // 未知错误 HTTP 500
    // INVALID_ARGUMENT = 3; // 无效的参数 HTTP 400
    // DEADLINE_EXCEEDED = 4; // 超时 HTTP 504
    // NOT_FOUND = 5; // 未找到 HTTP 404
    // ALREADY_EXISTS = 6; // 已存在 HTTP 409
    // PERMISSION_DENIED = 7; // 权限不足 HTTP 403
    // UNAUTHENTICATED = 16; // 未认证 HTTP 401
    // RESOURCE_EXHAUSTED = 8; // 资源耗尽 HTTP 429
    // FAILED_PRECONDITION = 9; // 前置条件失败 HTTP 412
    // ABORTED = 10; // 操作被中止 HTTP 409
    // OUT_OF_RANGE = 11; // 超出范围 HTTP 416
    // UNIMPLEMENTED = 12; // 未实现 HTTP 501
    // INTERNAL = 13; // 内部错误 HTTP 500
    // UNAVAILABLE = 14; // 服务不可用 HTTP 503
    // DATA_LOSS = 15; // 数据丢失 HTTP 500

    impl Status {
        pub fn http_code(&self) -> i32 {
            match self.code {
                0 => 200,
                1 => 404,
                2 => 500,
                3 => 400,
                4 => 504,
                5 => 404,
                6 => 409,
                7 => 403,
                8 => 429,
                9 => 412,
                10 => 409,
                11 => 416,
                12 => 501,
                13 => 500,
                14 => 503,
                15 => 500,
                _ => 500,
            }
        }
    }

    impl From<Code> for Status {
        fn from(code: Code) -> Self {
            Status {
                code: code as i32,
                ..Default::default()
            }
        }
    }
}

#[cfg(feature = "connect")]
pub mod connect {
    include!(concat!(env!("OUT_DIR"), "/vela.connect.rs"));
}

#[cfg(feature = "table")]
pub mod table {
    include!(concat!(env!("OUT_DIR"), "/vela.table.rs"));
}
