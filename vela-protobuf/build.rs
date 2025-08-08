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

    // 收集需要编译的 proto 文件
    let mut proto_files = Vec::new();
    let include_dirs = vec![proto_dir];
    // 根据 feature 有条件地包含不同的 proto 文件
    if cfg!(feature = "common") {
        proto_files.push("../apis/vela/common/code.proto");
        proto_files.push("../apis/vela/common/status.proto");
        proto_files.push("../apis/vela/common/api.proto");
        proto_files.push("../apis/vela/common/metadata.proto");
        println!("cargo:rustc-cfg=feature=\"common\"");
    }

    if cfg!(feature = "connect") {
        proto_files.push("../apis/vela/connect/connect.proto");
        println!("cargo:rustc-cfg=feature=\"connect\"");
    }

    if cfg!(feature = "table") {
        proto_files.push("../apis/vela/table/table.proto");
        println!("cargo:rustc-cfg=feature=\"table\"");
    }

    // 如果没有启用任何 feature，至少编译 common
    if proto_files.is_empty() {
        proto_files.push("../apis/vela/common/code.proto");
        proto_files.push("../apis/vela/common/status.proto");
    }

    // 编译 proto 文件
    if !proto_files.is_empty() {
        config.compile_protos(&proto_files, &include_dirs)?;
    }

    // 告诉 cargo 在 proto 文件变化时重新运行 build script
    println!("cargo:rerun-if-changed=../apis/vela/");
    for file in &proto_files {
        println!("cargo:rerun-if-changed={}", file);
    }

    Ok(())
}
