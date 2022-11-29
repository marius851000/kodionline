{ pkgs ? import <nixpkgs> {},
naersk ? import (pkgs.fetchFromGitHub {
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

	kodi-dl = pkgs.fetchFromGitHub {
		owner = "marius851000";
		repo = "kodi-dl";
		rev = "d03c12102a5328fe50ef74727aa92ebf4afc7669";
		sha256 = "G/mE9LcvaMTnXUpbuklc8MSZLziVZ5CoFkOZEVEN+sc=";
	};
in
pkgs.stdenv.mkDerivation {
	name = "kodionline";

	nativeBuildInputs = with pkgs; [
		python3
#		urlencode
		pkg-config
		bubblewrap
	] ++ (with pkgs.python3Packages; [
		chardet
		mock
		lxml
		urllib3
		pkgs.openssl
		certifi
		idna
	]);

	NIX_ENFORCE_PURITY=0;

	shellHook = ''
		export PYTHONPATH=$PYTHONPATH:${kodi-dl}
	'';
}
