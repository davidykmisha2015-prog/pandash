{
  description = "pandash - A custom minimalist shell written in Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
          ];

          # Забезпечуємо правильне середовище для компіляції низькорівневих пакетів
          shellHook = ''
            echo "Rust version: $(rustc --version)"
          '';
        };
      }
    );
}
