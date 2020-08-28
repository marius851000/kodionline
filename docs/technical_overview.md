# technical overview

This website is programmed using the rust programming language and the web server library rocket. It also use multiple other library.

An important know before reading this is knowing the client-server architecture: there are code ran by the server (the computer of the one who host the website), and code ran by the client (your computer, more exactly the web browser).

Another important thing to understand is that the original data aren't stored on the server of this website, but by server ran by various different people (the owner of replay site, video sites, etc).

## getting the metadata
All the metadata are obtained by the server. It use kodi addons to download metadata from the original site. Kodi is an open source software that can browser website using the same presentation than this site (and more). Kodi addons are wrote in the Python programming language.

I use kodi-dl to execute the addon. It is then the addon task to download and parse the various metadata for a content, for example the list of subfolder, the url of videos/audios, name of those...

kodi-dl is a recreate of the kodi interface using only python. It can also be used for other task, the main other use case is creating a copy of content accessible by kodi addons.

## serving the website
The server-side code of the website is made using another programming language, rust (not to be confused with the game with the same name).

When you want to display a page, it will:
- call the python kodi addon, let the addon download the metadata, and get back the metadata
- generate the view for those data.

It will, notably, tell if the url is a media or a folder. In case it is folder of an audio plugin, it will also add a music player for the audios element (usually music).

## serving the media
each time the client want to play a media, it will call an url from the site. The server will then redirect the client to the good url. This is put in place for two reason (rather than giving directly the url to the client when displaying the page):
- It allow to server both local (file stored on the server of the owner of this website) and extern (file stored on server of others)  to be played simply.
- It allow to send the page to the client even when the url of the media is unknown. This is particularly usefull for the media player, which may have tens of music, so tens of url, and tens of download to do before knowing the url of all the media, and being able to send the page to the client.
