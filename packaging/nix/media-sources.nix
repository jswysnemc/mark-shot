/**
  【Nix媒体】【固定来源】按 fetchurl 返回经过摘要校验的媒体归档，构建阶段不下载源码
*/
{ fetchurl }:
{
  ffmpeg = fetchurl {
    url = "https://ffmpeg.org/releases/ffmpeg-9.0.tar.xz";
    sha256 = "7f607a00dd0d28a729d5a4811205812eef01cf6ef6155025febb6f36a9062d52";
  };
  openh264 = fetchurl {
    url = "https://codeload.github.com/cisco/openh264/tar.gz/refs/tags/v2.6.0";
    sha256 = "558544ad358283a7ab2930d69a9ceddf913f4a51ee9bf1bfb9e377322af81a69";
  };
  nv-codec-headers = fetchurl {
    url = "https://github.com/FFmpeg/nv-codec-headers/archive/refs/tags/n13.0.19.0.tar.gz";
    sha256 = "86d15d1a7c0ac73a0eafdfc57bebfeba7da8264595bf531cf4d8db1c22940116";
  };
}
