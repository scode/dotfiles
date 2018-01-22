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
  };
}
