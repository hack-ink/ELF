pub fn render_schema(vector_dim: u32) -> String {
    include_str!("../../../sql/init.sql").replace("<VECTOR_DIM>", &vector_dim.to_string())
}
