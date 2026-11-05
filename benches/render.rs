use std::hint::black_box;
use std::time::Instant;

fn main() {
    let cases = [
        (
            "plain",
            "# Heading\n\nA paragraph with **bold**, a [link](https://example.com), and a table.\n\n| A | B |\n| - | - |\n| 1 | 2 |",
        ),
        (
            "nested-containers",
            ":::note\n:::tabs\n::tab label=\"One\"\n:::details summary=\"Details\"\nNested **content**.\n:::\n::\n:::",
        ),
        (
            "defines-and-extensions",
            ":::define\nname: \"MDRS\"\ncount: 3\n:::\n\nHello ${name}, ${count}.\n\n:::math\nx^2 + 1\n:::\n\n```mermaid\nflowchart LR\nA --> B\n```",
        ),
        (
            "sanitization-and-links",
            "<script>alert(1)</script><a href=\"https://example.com\">external</a>\n\n<a onclick=\"bad()\" href=\"/local\">local</a>",
        ),
    ];

    for (name, source) in cases {
        let samples = 7;
        let iterations = 1_000;
        let mut timings = Vec::with_capacity(samples);
        for _ in 0..samples {
            let started = Instant::now();
            for _ in 0..iterations {
                let result = mdrs::render_markdown(black_box(source)).expect("benchmark render");
                black_box(result);
            }
            timings.push(started.elapsed());
        }
        timings.sort_unstable();
        let median = timings[samples / 2];
        println!(
            "{name}: {} ns/op ({} iterations, {} samples)",
            median.as_nanos() / iterations,
            iterations,
            samples
        );
    }
}
