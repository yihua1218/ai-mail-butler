use samotop::mail::*;

fn main() {
    let dir = Dir::new("data/mail_spool".into());
    let mail = Builder::default().using(dir);
}
