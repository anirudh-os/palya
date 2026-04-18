use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

#[test]
fn test_end_to_end_build() {
    // Create a temp directory 
    let temp_dir = tempfile::tempdir().unwrap();
    let input_dir = temp_dir.path().join("content");
    let output_dir = temp_dir.path().join("dist");
    let templates_dir = temp_dir.path().join("templates");

    // Setup Dummy Files
    fs::create_dir(&input_dir).unwrap();
    fs::create_dir(input_dir.join("content")).unwrap();
    fs::create_dir(&templates_dir).unwrap();
    
    fs::write(input_dir.join("content").join("test.md"), "---\ntitle: T\n---\n# Hi").unwrap();
    fs::write(templates_dir.join("page.j2"), "{{ page.content }}").unwrap();

    // Run the binary
    let mut cmd = cargo_bin_cmd!("palya");
    cmd.arg("--input").arg(&input_dir)
       .arg("--output").arg(&output_dir)
       .arg("--templates").arg(&templates_dir)
       .assert()
       .success(); // Ensure exit code 0

    // Verify Output
    let expected_file = output_dir.join("test/index.html");
    assert!(expected_file.exists());
    
    let content = fs::read_to_string(expected_file).unwrap();
    assert!(content.contains("<h1>Hi</h1>"));
}