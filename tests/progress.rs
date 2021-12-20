#[test]
fn tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/01-parse.rs");
    // t.compile_fail("tests/02-reject-too-many-type-parameters.rs");
}
