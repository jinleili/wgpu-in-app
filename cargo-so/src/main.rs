use cargo_subcommand::Subcommand;
use ndk_build::cargo::cargo_ndk;
use ndk_build::error::NdkError;
use ndk_build::ndk::Ndk;
use ndk_build::target::Target;

fn main() {
    let args = std::env::args();
    match Subcommand::new(args, "so", |_, _| Ok(false)) {
        Ok(cmd) => match cmd.cmd() {
            "build" | "b" => {
                let ndk = Ndk::from_env().unwrap();
                let build_targets = if let Some(target) = cmd.target() {
                    vec![Target::from_rust_triple(target).ok().unwrap()]
                } else {
                    vec![
                        Target::Arm64V8a,
                        Target::ArmV7a,
                        Target::X86,
                        Target::X86_64,
                    ]
                };
                for target in build_targets {
                    let triple = target.rust_triple();
                    // setting ar, linker value
                    let mut cargo = cargo_ndk(&ndk, target, 24).unwrap();
                    cargo.arg("rustc");
                    if cmd.target().is_none() {
                        cargo.arg("--target").arg(triple);
                    }
                    cargo.args(cmd.args());
                    if ndk.build_tag() > 7272597 {
                        if !cmd.args().contains(&"--".to_owned()) {
                            cargo.arg("--");
                        }
                        let gcc_link_dir = cmd.target_dir().join("gcc-temp-extra-link-libraries");
                        let _ = std::fs::create_dir_all(&gcc_link_dir);
                        std::fs::write(gcc_link_dir.join("libgcc.a"), "INPUT(-lunwind)")
                            .expect("Failed to write");
                        println!("gcc_dir: {:?}", gcc_link_dir);
                        let test_dir = "/Users/lijinlei/Rust/android-ndk-rs/target/cargo-apk-temp-extra-link-libraries";
                        // cargo.arg("-L").arg(gcc_link_dir);
                        cargo.arg("-L").arg(test_dir);
                    }

                    if !cargo.status().unwrap().success() {
                        println!("{:?}", NdkError::CmdFailed(cargo));
                    }
                }
            }
            _ => {}
        },
        Err(_) => {}
    };
}
