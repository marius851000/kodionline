{ pkgs ? import <nixpkgs> {} }:

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
	];

	preConfigure = ''
		export PYTHONPATH=$PYTHONPATH:/home/marius/kodi-dl
		fish
		exit
	'';
}
