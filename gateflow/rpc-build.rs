fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(false) // 只做 server 就关掉 client
        .compile_protos(&["proto/v1/service.proto"], &["proto"])?;
    Ok(())
}
