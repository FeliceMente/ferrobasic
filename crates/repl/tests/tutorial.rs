use ferro_repl::run;

#[test]
fn interactive_tutorial_transcripts_are_real() {
    let tutorial = include_str!("../../../docs/tutorial-repl.md");
    let mut count = 0;
    for block in tutorial.split("```repl\n").skip(1) {
        let block = block.split("```").next().unwrap();
        let (mut input, mut expected_out, mut expected_err) =
            (String::new(), String::new(), String::new());
        for line in block.lines() {
            if let Some(code) = line
                .strip_prefix("> ")
                .or_else(|| line.strip_prefix("... "))
            {
                input.push_str(code);
                input.push('\n');
            } else if line.starts_with("<repl:") {
                expected_err.push_str(line);
                expected_err.push('\n');
            } else {
                expected_out.push_str(line);
                expected_out.push('\n');
            }
        }
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run(&mut input.as_bytes(), &mut out, &mut err, true), 0);
        assert_eq!(String::from_utf8(out).unwrap(), expected_out, "{input}");
        let err = String::from_utf8(err)
            .unwrap()
            .replace("... ", "")
            .replace("> ", "");
        assert_eq!(err, expected_err, "{input}");
        count += 1;
    }
    assert_eq!(count, 6);
}

#[test]
fn piped_tutorial_transcripts_stop_at_errors() {
    let tutorial = include_str!("../../../docs/tutorial-repl.md");
    for (i, block) in tutorial.split("```repl-script\n").skip(1).enumerate() {
        let input = block.split("```").next().unwrap();
        let output = block
            .split("```repl-output\n")
            .nth(1)
            .unwrap()
            .split("```")
            .next()
            .unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(
            run(&mut input.as_bytes(), &mut out, &mut err, false),
            if i == 0 { 0 } else { 1 }
        );
        assert_eq!(String::from_utf8(out).unwrap(), output);
        if i == 1 {
            assert!(
                String::from_utf8(err)
                    .unwrap()
                    .contains("NumberFormatException")
            );
        }
    }
}
