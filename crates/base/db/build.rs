/// A new migration file must rebuild the crate: `sqlx::migrate!` embeds the directory.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
