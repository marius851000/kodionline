{ pkgs ? import <nixpkgs> {}, naersk ? import (pkgs.fetchFromGitHub {
	owner = "nmattia";
	repo = "naersk";
	rev = "529e910a3f423a8211f8739290014b754b2555b6";
	sha256 = "3pDN/W17wjVDbrkgo60xQSb24+QAPQ7ulsUq5atNni0=";
})}:

let
	naersk-lib = pkgs.callPackage naersk { };

	urlencode = naersk-lib.buildPackage (pkgs.fetchFromGitHub {
		owner = "dead10ck";
		repo = "urlencode";
		sha256 = "iiebonXsCLZDsUCoWZ9zazDR+lpNQHNrb+vYJ6M8qVA=";
		rev = "0.1.2";
	});
in
pkgs.stdenv.mkDerivation {
	name = "kodionline";
	propagatedBuildInputs = with pkgs.python2Packages; [
		chardet
		mock
		lxml
		urllib3
	];

	nativeBuildInputs = with pkgs; [
		fish
		python2
		urlencode
	];
 	
	NIX_ENFORCE_PURITY=0;

	shellHook = ''
		export PYTHONPATH=$PYTHONPATH:/home/marius/kodi-dl
		fish
		exit
	'';
}
