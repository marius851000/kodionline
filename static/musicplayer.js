var music_players = document.getElementsByClassName("audiopreview");

for (player of music_players) {
	player.addEventListener("mouseover", function(e) {
		e.target.preload = "auto";
	}); //TODO: maybe use the whole tile choose to preload
	player.addEventListener("playing", function(e) {
		for (player of music_players) {
			if (player.getAttribute("audiopreview_nb") != e.target.getAttribute("audiopreview_nb")) {
				player.pause();
				player.fastSeek(0);
			};
		};
	});
	player.addEventListener("ended", function(e) {
		e.target.fastSeek(0);
		var next_id = Number(e.target.getAttribute("audiopreview_nb")) + 1;
		for (player of music_players) {
			if (player.getAttribute("audiopreview_nb") == String(next_id)) {
				player.play();
			};
		};
	});
	player.addEventListener("seeked", function(e) {
		if (e.target.currentTime > 0.1) { //check to prevent to catch fastSeek(0)
			e.target.play();
		};
	})
}
