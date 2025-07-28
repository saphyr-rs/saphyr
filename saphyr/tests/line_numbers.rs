use saphyr::LoadableYamlNode;
use saphyr::MarkedYamlOwned;

#[test]
fn example_for_line_numers() {
    // Notice how the lines start at 1 not at 0.
    // The lines are absolute to the entire file,
    // meaning that even when a second document
    // starts, the reported line numbers are high.

    let s = r#"
        [ 1]# from yaml-cpp example
        [ 2]- name: Ogre
        [ 3]  position: [0, 5, 0]
        [ 4]  powers:
        [ 5]    - name: Club
        [ 6]      damage: 10
        [ 7]    - name: Fist
        [ 8]      damage: 8
        [ 9]- name: Dragon
        [10]  position: [1, 0, 10]
        [11]  powers:
        [12]    - name: Fire Breath
        [13]      damage: 25
        [14]    - name: Claws
        [15]      damage: 15
        [16]- name: Wizard
        [17]  position: [5, -3, 0]
        [18]  powers:
        [19]    - name: Acid Rain
        [20]      damage: 50
        [21]    - name: Staff
        [22]      damage: 3
        [23]---
        [24]- name: Elf
        [25]  position: [-3, -8, 7]
        [26]  powers:
        [27]    - name: Arrow
        [28]      damage: 35
        [29]    - name: Dagger
        [30]      damage: 10
        "#
    .lines()
    .map(|line| line.trim())
    .filter(|line| !line.is_empty())
    .map(|line| line.chars().skip("[01]".len()).collect::<String>())
    .collect::<Vec<_>>()
    .join("\n");

    let docs = MarkedYamlOwned::load_from_str(&s).unwrap();

    let first = docs[0].clone();
    assert_eq!(first.span.start.line(), 2);
    assert_eq!(first.span.end.line(), 23);
    assert_eq!(first.span.start.col(), 0);
    assert_eq!(first.span.end.col(), 0);

    let name_node = first.data[0].data["name"].clone();
    assert_eq!(name_node.data.as_str().unwrap(), "Ogre");
    assert_eq!(name_node.span.start.line(), 2);
    assert_eq!(name_node.span.end.line(), 2);
    assert_eq!(name_node.span.start.col(), 8);
    assert_eq!(name_node.span.end.col(), 12);

    let power_claws = first.data[1].data["powers"].data[1].clone();
    assert_eq!(power_claws.span.start.line(), 14);
    assert_eq!(power_claws.span.end.line(), 16);
    assert_eq!(power_claws.span.start.col(), 6);
    assert_eq!(power_claws.span.end.col(), 0);

    let second = docs[1].clone();
    assert_eq!(second.span.start.line(), 24);
    assert_eq!(second.span.end.line(), 31);
    assert_eq!(second.span.start.col(), 0);
    assert_eq!(second.span.end.col(), 0);
}
