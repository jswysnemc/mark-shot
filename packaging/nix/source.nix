/**
  【Nix打包】【源码筛选】按 lib 与 root 返回程序和媒体的独立源集，排除缓存及 Nix 描述文件
*/
{ lib, root }:
let
  applicationFiles = lib.fileset.unions (
    map (name: root + "/${name}") [
      "Cargo.toml"
      "Cargo.lock"
      "build.rs"
      "rust-toolchain.toml"
      "LICENSE-MIT"
      "LICENSE-APACHE"
      "src"
      "crates"
      "assets"
      "tests"
      "examples"
      "scripts"
      "packaging"
      "release"
      "docs"
      ".github"
      "debian"
    ]
  );
in
{
  application = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.difference applicationFiles (root + "/packaging/nix");
  };
  media = lib.fileset.toSource {
    inherit root;
    fileset = lib.fileset.unions (
      map (name: root + "/${name}") [
        "crates/shot-media/native"
        "scripts/media/build-libraries.sh"
        "scripts/ci-download.py"
        "scripts/packaging/fetch_media_sources.py"
        "scripts/packaging/media_checksums.py"
        "scripts/packaging/artifact_metadata.py"
      ]
    );
  };
}
