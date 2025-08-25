include!(concat!(env!("OUT_DIR"), "/vela.common.rs"));

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
