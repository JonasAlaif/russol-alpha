use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

pub fn replace_with_sln<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, sln: String, keep_newlines: bool) {
    let span: rustc_span::Span = tcx.source_span(def_id.expect_local());
    let sm = tcx.sess.source_map();
    let flines = sm.span_to_lines(span).unwrap();
    let fname = if let rustc_span::FileName::Real(rustc_span::RealFileName::LocalPath(fname)) =
        &flines.file.name
    {
        fname
    } else {
        panic!()
    };
    let src = std::fs::read_to_string(fname).unwrap();
    let (first, last) = (flines.lines.first().unwrap(), flines.lines.last().unwrap());
    let (fline, lline) = (first.line_index, last.line_index + 1);
    let mut method = src.split('\n').into_iter().skip(fline).take(lline - fline);
    // let debug = src.split('\n').into_iter().skip(fline).take(lline - fline).fold(String::new(), |acc, line| acc + line + "\n");
    // eprintln!("Fn ({:?} {:?}):\n{debug}", first, last);
    let line1 = method.next().unwrap();
    // let mut last_line_len = line1.len();
    // line1[first.start_col.0..].to_string()
    let method = method.fold(line1.to_string(), |acc, line| {
        // last_line_len = line.len();
        acc + "\n" + line
    });
    // eprintln!("{last_line_len} vs {}", last.end_col.0);
    // let to_cut_off = last_line_len-last.end_col.0;
    // let method = &method[..method.len()-to_cut_off];
    // let fn_sig = format!("fn {}", tcx.item_name(def_id).as_str());
    // eprintln!("Fn sig: {fn_sig} and method:\n{method}");
    // let mut split = method.split(&fn_sig);
    // let body = if let (_, Some(body), None) = (split.next(), split.next(), split.next()) {
    //     body
    // } else { panic!() };
    let mut braces = 0;
    let mut curly_braces = 0;
    let mut in_body = 0;
    let mut in_comment = 0;
    let body: String = method
        .chars()
        .into_iter()
        .filter(|c| {
            if in_comment == 1 {
                if *c == '/' {
                    in_comment = 2
                } else {
                    in_comment = 0
                }
            } else if *c == '\n' {
                in_comment = 0
            }
            if in_comment == 0 {
                match c {
                    '/' => in_comment = 1,
                    '(' => braces += 1,
                    ')' => braces -= 1,
                    '{' => {
                        if braces == 0 && curly_braces == 0 {
                            in_body += 1;
                        }
                        curly_braces += 1
                    }
                    '}' => {
                        curly_braces -= 1;
                        if braces == 0 && curly_braces == 0 {
                            in_body += 1;
                            return true;
                        }
                    }
                    _ => (),
                }
            };
            in_body == 1
        })
        .collect();
    assert_eq!(
        in_body, 2,
        "Couldn't find {{ body }} in ({span:?})\n{method}"
    );
    assert_eq!(
        method.matches(&body).count(),
        1,
        "Looking for {body} in\n{method}"
    );
    let sln = sln.replace('\n', &format!("\n{}", " ".repeat(first.start_col.0)));
    let mut new_method = method.replace(&body, &format!("{{{}}}", sln));
    if keep_newlines {
        let (new_method_lines, method_lines) = (
            new_method.matches('\n').count(),
            method.matches('\n').count(),
        );
        match new_method_lines.cmp(&method_lines) {
            std::cmp::Ordering::Less => {
                for _ in 0..(method_lines - new_method_lines) {
                    new_method += "\n";
                }
            }
            std::cmp::Ordering::Equal => (),
            std::cmp::Ordering::Greater => {
                new_method = new_method.replacen('\n', "", new_method_lines - method_lines);
            }
        }
        assert_eq!(
            new_method.matches('\n').count(),
            method.matches('\n').count()
        );
    }

    // let new_src = src.replace(&method, &new_method);
    let pre = src.split('\n').take(fline);
    let post = src.split('\n').skip(lline);
    let mut total = pre.chain(new_method.split('\n')).chain(post);
    let init = total.next().unwrap().to_string();
    let new_src = total.fold(init, |acc, line| acc + "\n" + line);
    // println!("{src}\nVS\n{new_src}");
    if keep_newlines {
        assert_eq!(
            src.matches('\n').count(),
            new_src.matches('\n').count(),
            "{fname:?} ({:?} {:?})\n{src}\n-- VS --\n{new_src}",
            first,
            last
        );
    }
    std::fs::write(fname, &new_src).unwrap();
}
