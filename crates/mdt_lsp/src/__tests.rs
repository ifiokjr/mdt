use super::*;

#[test]
fn you_can_use_a_test() {
  let content = r#"This is something!<!-- ={exampleName} -->
  Placeholder text
  <!-- {/exampleName} -->"#;
  let result = get_node_from_content(content).unwrap();
  println!("{result:#?}");
}
