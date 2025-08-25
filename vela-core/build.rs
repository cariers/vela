use std::env;
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // 获取 proto 文件目录
    let proto_dir = "../apis";
    // 构建 prost 配置
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);

    // 告诉 cargo 在 proto 文件变化时重新运行 build script
    println!("cargo:rerun-if-changed=../apis/vela/");

    // 收集需要编译的 proto 文件
    let mut proto_files = Vec::new();
    let include_dirs = vec![proto_dir];

    proto_files.push("../apis/vela/common/code.proto");
    proto_files.push("../apis/vela/common/status.proto");
    proto_files.push("../apis/vela/common/api.proto");
    proto_files.push("../apis/vela/common/metadata.proto");
    proto_files.push("../apis/vela/common/code.proto");
    proto_files.push("../apis/vela/common/status.proto");
    config.compile_protos(&proto_files, &include_dirs)?;

    Ok(())
}
