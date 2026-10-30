use std::fs;

const TEMPLATE: &str = std::include_str!("../Template.svg");
const USAGE: &str = "Generate Wordle board image SVG for a specific set of guesses and responses.
  USAGE: wordle-svg <out-file-path> (<guess> <response>)+
  ex: wordle-svg wordle-cs-are.svg clint bbbbb soare bbyyy
  Guesses and Responses must be five letters long.
  Response letters: b = Black, y = Yellow, g = Green.
";

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("{}", USAGE);
        return;
    }

    let output_file_path = &args[1];
    let mut args = &args[2..];

    // Start the output SVG; set the height based on the number of guesses
    let row_count = args.len() / 2;
    let mut output = TEMPLATE.replace("height=\"127\"", &format!("height=\"{}\"", 5 + row_count * 61));

    let mut row = 1;
    while args.len() > 1 {
        let guess = args[0].to_ascii_uppercase();
        let response = &args[1].to_ascii_lowercase();

        output.push_str(&format!("\n<g class=\"r{row}\">"));

        for (col, (guess_char, response_char)) in guess.chars().zip(response.chars()).enumerate() {
            output.push_str(&format!("\n<g class=\"c{}\">\n", col + 1));
            output.push_str(&format!("<rect class=\"t {response_char}\" />\n"));
            output.push_str(&format!("<text>{guess_char}</text>\n"));
            output.push_str("</g>\n");
        }

        output.push_str("</g>\n");

        row += 1;
        args = &args[2..];
    }

    output.push_str("\n\n</svg>");

    fs::write(output_file_path, output).expect("Unable to write output SVG!");
}