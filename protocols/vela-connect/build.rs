use std::env;
use std::io::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // 获取 proto 文件目录
    let proto_dir = "../../apis/vela";
    let include_dirs = vec![proto_dir];
    let mut proto_files = Vec::new();
    // 构建 prost 配置
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    proto_files.push(format!("{}/connect/connect.proto", proto_dir));
    // 编译 proto 文件
    config.compile_protos(&proto_files, &include_dirs)?;
    // 告诉 cargo 在 proto 文件变化时重新运行 build script
    println!("cargo:rerun-if-changed={}", proto_dir);
    for file in &proto_files {
        println!("cargo:rerun-if-changed={}", file);
    }

    Ok(())
}
