![Logo](assets/pandash.png)

## PandaSH
pandash is the new sh for linux distros.

this project writen on rust(what you see).

## Features
Sorry but now this project doesn't have any features but in future MAYBE created a very cool features :3.

## Download
to download this project now you need to clone github repository by this command:
```bash
git clone https://github.com/davidykmisha2015-prog/pandash.git
cd pandash
cargo build --release
cargo run
```
and binary created in ./target/releases/pandash
## If you uses nixos
in nixos this command works but if you want download it in your system you need to add pandash into your flake.nix and configuration.nix:
```nix
{
  inputs = {
    pandash.url = "github:davidykmisha2015-prog/pandash";
  };
  outputs = { self, nixpkgs, pandash, ... }@inputs: {
    # your another settings...
  };
}
```
## 2: Add it to your configuration.nix
```nix
environment.systemPackages = [
  inputs.pandash.packages.${pkgs.system}.default
];
```
Aaaaand rebuild your system!
## Support
Guys i know you're love open source and free software.But do you can support me on Ko-fi? This is very big help for me! okay here's the button:
<a href='https://ko-fi.com/romixcaca' target='_blank'>
  <img height='36' style='border:0px;height:36px;' src='https://storage.ko-fi.com/cdn/kofi3.png?v=3' border='0' alt='Buy Me a Coffee at ko-fi.com' />
</a>

