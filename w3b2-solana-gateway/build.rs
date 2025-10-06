fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().build_server(true).compile(
        &["../proto/types.proto", "../proto/gateway.proto"], // The file to compile
        &["../proto"],                                       // The directory to search in
    )?;
    Ok(())
}
