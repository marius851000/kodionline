{
  description = "kodionline";

  inputs.fenix = {
    url = "github:nix-community/fenix";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/ea5dd5d6affb9d70071c09e8e18e6afbb15635a8"; # with the change.

  inputs.kodidl = {
    url = "github:marius851000/kodi-dl";
    flake = false;
  };

  outputs = { self, nixpkgs, flake-utils, fenix, kodidl }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        pypack = pkgs.python3Packages;
        toolchain = fenix.packages.${system}.minimal.toolchain;
        rustPlatform = (pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        });
      in rec {
        packages = rec {
          kodionline_unwrapped = rustPlatform.buildRustPackage {
            pname = "kodionline-unwrapped";
            version = "git";

            src = ./.;

            postPatch = ''
              substituteInPlace ./kodionline/src/main.rs \
                --replace \"static\" \"$out/share/kodionline/static\"
            '';

            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];

            postInstall = "
              mkdir -p $out/share/kodionline
              cp -r ${./static} $out/share/kodionline/static
            ";

            cargoLock.lockFile = ./Cargo.lock;
          };

          kodionline = pkgs.stdenv.mkDerivation {
            pname = "kodionline-wrapped";
            version = "git";

            dontUnpack = true;

            nativeBuildInputs = with pkgs; [
                python3
                pkg-config
                bubblewrap
                makeWrapper
              ] ++ (with pkgs.python3Packages; [
                chardet
                mock
                lxml
                urllib3
                pkgs.openssl
                certifi
                idna
            ]);

            installPhase = ''
              mkdir -p $out/bin
              makeWrapper ${kodionline_unwrapped}/bin/kodionline $out/bin/kodionline \
                --prefix PYTHONPATH : $PYTHONPATH:${kodidl} \
                --prefix PATH : ${pkgs.python3}/bin
            '';
          };
        };
        defaultPackage = packages.kodionline;
      }
    );
}