pub fn generate_key(length: usize) -> String {
    genrs_lib
        ::encode_key(genrs_lib::generate_key(length), genrs_lib::EncodingFormat::Base64)
        .expect("Key generation should succeed.")
}
