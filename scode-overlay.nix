self: super:

{
  # Install overlay by copying/symlinking this to ~/.config/nixpkgs/overlays/scode.nix
  userPackages = super.userPackages or {} // {
    # Must not forget nix itself or nix-env will disappear when nix-rebuild-scode is called.
    nix = super.nix;

    ripgrep = super.ripgrep;

    nix-rebuild-scode = super.writeScriptBin "nix-rebuild-scode"
      ''
        #!${super.stdenv.shell}
        exec nix-env -f '<nixpkgs>' -r -iA userPackages
      '';

    saltybox = super.buildGoPackage rec {
      name = "saltybox-${version}";
      version = "1.0.0";

      goPackagePath = "github.com/scode/saltybox";

      src = super.fetchFromGitHub {
        owner = "scode";
        repo = "saltybox";
        rev = "v${version}";
        sha256 = "0a2p2hicw71vzycxg4xafl7xj4y3ky7h9v67g1b1ai1pwabx08sc";
      };

      buildFlags = "--tags release";
     };
  };
}
