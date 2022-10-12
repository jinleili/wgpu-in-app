use cargo_subcommand::Subcommand;
use clap::Parser;
use ndk_build::cargo::cargo_ndk;
use ndk_build::error::NdkError;
use ndk_build::ndk::Ndk;
use ndk_build::target::Target;

#[derive(Parser)]
struct Cmd {
    #[clap(subcommand)]
    so: SoCmd,
}

#[derive(clap::Subcommand)]
enum SoCmd {
    /// Helps cargo build so for Android
    So {
        #[clap(subcommand)]
        cmd: SoSubCmd,
    },
}
#[derive(Parser)]
struct Args {
    #[clap(flatten)]
    subcommand_args: cargo_subcommand::Args,
    #[clap(short, long)]
    device: Option<String>,
}

#[derive(clap::Subcommand)]
#[clap(trailing_var_arg = true)]
enum SoSubCmd {
    /// Compile the current package and create an apk
    #[clap(visible_alias = "b")]
    Build {
        #[clap(flatten)]
        args: Args,
    },
    /// Invoke `cargo` under the detected NDK environment
    #[clap(name = "--")]
    Ndk {
        cargo_cmd: String,
        #[clap(flatten)]
        args: Args,
    },
}

fn main() {
    let Cmd {
        so: SoCmd::So { cmd },
    } = Cmd::parse();
    match cmd {
        SoSubCmd::Build { args } => {
            let cmd = Subcommand::new(args.subcommand_args).unwrap();
            let ndk = Ndk::from_env().unwrap();
            let build_targets = if let Some(target) = cmd.target() {
                vec![Target::from_rust_triple(&target).ok().unwrap()]
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
                let mut cargo = cargo_ndk(&ndk, target, 24, cmd.target_dir()).unwrap();
                cargo.arg("rustc");
                if cmd.target().is_none() {
                    cargo.arg("--target").arg(triple);
                }
                cmd.args().apply(&mut cargo);

                // ndk-build 0.8 already implemented following logic：
                // if ndk.build_tag() > 7272597 {
                //     // if !cmd.args().contains(&"--".to_owned()) {
                //     //    cargo.arg("--");
                //     // }
                //     let gcc_link_dir = cmd.target_dir().join("gcc-temp-extra-link-libraries");
                //     let _ = std::fs::create_dir_all(&gcc_link_dir);
                //     std::fs::write(gcc_link_dir.join("libgcc.a"), "INPUT(-lunwind)")
                //         .expect("Failed to write");
                //     cargo.arg("\x1f-L\x1f").arg(gcc_link_dir);
                // }

                if !cargo.status().unwrap().success() {
                    println!("{:?}", NdkError::CmdFailed(cargo));
                }
            }
        }
        SoSubCmd::Ndk { .. } => {}
    };
}
