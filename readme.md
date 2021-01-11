## kodi online
This is a website that allow you to browser kodi addon with a web browser. It made it so it can be acessed publically on internet, but will need to check the security.

The python code is sandbox with bubblewrap, minimizing risk with faulty addon.

dependancies: xbmcemu from kodi-dl

### how to run
with nix installed, and the configuration writtent in config.json (here is an exemple):

note: you will need to first install any of those addon first. If they are not present they can't be used.

You can install them either via kodi (for radio, jambmc and arte replay) or my github page (for mlp-france and need for ponies). additionally, the youtube extension may work.

```json
{
	"plugins_to_show": [
		["mlp-france videos", "plugin://plugin.video.mlpfrance/?"],
		["mlp-france audios", "plugin://plugin.audio.mlpfrance/?"],
		["need for ponies videos", "plugin://plugin.video.needforponies/?"],
		["need for ponies audios", "plugin://plugin.audio.needforponies/?"],
		["arte replay", "plugin://plugin.video.arteplussept/?"],
		["jambmc", "plugin://plugin.audio.jambmc/?"],
		["radio", "plugin://plugin.audio.radio_de/?"]
	],
	"kodi_path": "~/.kodi",
	"python_command": "python2",
	"default_user_config": {
		"language_order": ["fr", "en"],
		"resolution_order": ["720p", "480p", "1080p", "360p"],
		"format_order": ["mp4", "webm"]
	},
	"allowed_path": [ "/home/marius/Vidéos" ]
}

```

(you may need to change some stuff, mainly allowed_path if you want to allow read access to some folder)
```bash
nix-shell --pure
rustup deafult nightly
cargo run --bin kodionline -- -c ./config.json
```

(the --pure flag is used to prevent conflict with the system installed python3 installation)

You could then browser this website on localhost:8000 (or some other url it will print in the console)

### additional information
This can run both python2 and python3 addon. most of these extension are however only tested with python2. need for ponies and mlpfrance are known to work with python3.

### screenshots

![main page](/screenshot/main_page.png)

![content list](/screenshot/list_content.png)

![music player](/screenshot/music_reading.png)
Music player will automatically play the next music when the previous one is finished, and does not preload every music when not necessary.

![language selection](/screenshot/language_selection.png)
I have made an extension to the kodi API that can be used with kodi-dl allowing to select the language of a content.


### technical information

This is writtin using rust, with (mainly) the rocket HTTP server and the Maud html constructor. That mean that every html content is generated with rust, and escape HTML by default.
