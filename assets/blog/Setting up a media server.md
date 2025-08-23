---
date: 2025-08-23
published: true
read_mins: 10
tags:
  - "#linux"
  - "#mediaserver"
  - "#docker"
---

## Setting up machine.

For me since im using the _amazing_ [proxmox](https://www.proxmox.com/), I will create a lxc container to hold my jellyfin server. Since jellyfin isnt that resource heavy, only disk space heavy. For me 20gb should be enough for my use case, if you are planning to store a lot of movies just remember an hour long movie is around 2-4gb in size, so work around that. For my os im going to use debian 12, since i happened to have that template on my system, I chose to set 8gb of ram and 4 virtual cpu cores since i have a lot of spare compute.

## Setting up ssh

We next want to set up ssh on the server, since otherwise we need to look at the servers monitor all the time which can get repetitive after a while.

> I got sick of that a _looong_ time ago.

You bet, time to get it installed. This is done by first installing openssh by running `apt install openssh-server`, then...

```
root@jellyfin-school:/var/lib/apt# apt install openssh-server
Reading package lists... Done
Building dependency tree... Done
Reading state information... Done
Package openssh-server is not available, but is referred to by another package.
This may mean that the package is missing, has been obsoleted, or
is only available from another source
However the following packages replace it:
  openssh-sftp-server

E: Package 'openssh-server' has no installation candidate
(cut)...
```

Huh?

> You forgot something.

Oh! oh. i need to `apt update` don't i?

> Bingo

Well then, run `apt update && apt upgrade -y` to update package lists _then_ run `apt install openssh-server`. After installing that, it gives us access to the sshd service, which when enabling it with `systemctl start sshd.service` allows us to then get the ip from the server and ssh from your own computer.

> ok, this sounds doable

Yeah it is! for me since the ip of my machine is `10.0.0.39` i will ssh to the machine via...

> Hold up a minute, what is that number?

What number?

> `10.0.0.39`

The ip address?

> Yeah that

What about it?

> How did you get it?

By runnng `ip a` and getting the ip?

> Can you show how?

Fine. After you set up your server, you should have access to the console to run commands. If you run the command `ip a` it should give you a output like this

```
root@jellyfin-school:/docker/jellyfin# ip a
1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN group default qlen 1000
    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
    inet 127.0.0.1/8 scope host lo
       valid_lft forever preferred_lft forever
    inet6 ::1/128 scope host noprefixroute
       valid_lft forever preferred_lft forever
2: eth0@if78: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc noqueue state UP group default qlen 1000
    link/ether bc:24:11:25:8f:c1 brd ff:ff:ff:ff:ff:ff link-netnsid 0
    inet 10.0.0.39/24 brd 10.0.0.255 scope global eth0
       valid_lft forever preferred_lft forever
    inet6 fe80::be24:11ff:fe25:8fc1/64 scope link
       valid_lft forever preferred_lft forever
```

> Umm, what.

Luckily we can ignore a lot of that mess, and find what we are looking for, firstly we ignore the `lo` section and focus on the one named `eth0@if78`, it will probably be named differently on your system but they usually are named something like `eth0`, `ens3` or `wlp82s0` for wifi.

```
2: eth0@if78: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc noqueue state UP group default qlen 1000
    link/ether bc:24:11:25:8f:c1 brd ff:ff:ff:ff:ff:ff link-netnsid 0
    inet 10.0.0.39/24 brd 10.0.0.255 scope global eth0
       valid_lft forever preferred_lft forever
    inet6 fe80::be24:11ff:fe25:8fc1/64 scope link
       valid_lft forever preferred_lft forever
```

> ok... thats still a lot of numbers, how is this helping?

Hold on, first off we can ignore everything down to the `inet` line, and everything after. That leaves us with:

```
inet 10.0.0.39/24 brd 10.0.0.255 scope global eth0
```

The ip address is the number after `inet`, you can ignore the `/24` as you dont need it. That gives us:

```
10.0.0.39
```

> Right, so we just need to find the needle in the haystack

Luckily the needle is always in the same place.

Back on topic, we were about to `ssh` into the server right?

> yes

Well that's done relatively easily, as most systems come preinstalled with `ssh`, even windows.

Get the ip we just found, and the user you set when setting up the computer and run this command in your main computers terminal:

```
ssh user@ip
```

If everything worked it should come up this message:

```
➜  ~  ssh root@10.0.0.39
The authenticity of host '10.0.0.39 (10.0.0.39)' can't be established.
ED25519 key fingerprint is SHA256:FfyXLvd4bOQKJ2T7gCfSy1T4cSXOa8NOcEsvNgGmYDE.
This key is not known by any other names.
Are you sure you want to continue connecting (yes/no/[fingerprint])?
```

just type `yes` and hit enter, and it shouldn't appear again.
Now type in your password and...

```
(cut)...
root@jellyfin-school:~#
```

We are in 😎.

> hackerman.

Now that we are in, we can move onto actually setting up a media server.

### installing docker

In order to run jellyfin without having to move a lot of files into specific places, we should use docker and docker compose to run jellyfin. Don't worry its not too difficult.

Firstly, install curl with `apt install curl -y` in order to download the docker install script, you can download and run the script with the following commands, taken from dockers website.

1. `curl -fsSL https://get.docker.com -o get-docker.sh`
2. Make sure to allow the file to run:  
   `chmod +x get-docker.sh`
3. Run the installer with `./get-docker.sh`

Check that installed with `docker -v`:

```
root@jellyfin-school:~# docker -v
Docker version 28.3.2, build 578ccf6
```

Success!

Now that we have docker, we can move onto making the compose file for jellyfin.

## making the jellyfin docker compose

firstly lets check we have docker compose, it should have installed with the previous script:

```
root@jellyfin-school:~# docker compose
Usage:  docker compose [OPTIONS] COMMAND

(cut)...
```

> Well that's a good sign.

In order to run the jellyfin container, we can create whats known as a `docker compose` file, which contains instruction on how to run containers, which you can visualize as mini-computers with all the setup needed to run an app already installed. Lets walk through making a compose file for jellyfin.

Firstly we need to google and find a container for jellyfin, a good location for finding high-quality results is [linuxserver.io](htttps://linuxserver.io), who package and maintain a huge amount of docker images to use.

there! they have one we need https://docs.linuxserver.io/images/docker-jellyfin/

if we scroll down on this page they provide us with a working docker file, how kind. We can modify this to work for us and make sure we have required settings:

```yaml
---
services:
  jellyfin:
    image: lscr.io/linuxserver/jellyfin:latest
    container_name: jellyfin
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Etc/UTC
      - JELLYFIN_PublishedServerUrl=http://192.168.0.5 #optional
    volumes:
      - /path/to/jellyfin/library:/config
      - /path/to/tvseries:/data/tvshows
      - /path/to/movies:/data/movies
    ports:
      - 8096:8096
      - 8920:8920 #optional
      - 7359:7359/udp #optional
      - 1900:1900/udp #optional
    restart: unless-stopped
```

> _that's_, well... that's a lot

Yeah i agree, lets break this down to understand a bit easier.
Looking at this we can see its made of several different sections.

The first section is called services, this section is describing how each container needs to be set up.

```yaml
---
services:
(cut)...
```

looking a bit deeper we can see the section named jellyfin, which contains the config for jellyfin

```yaml
image: (cut)...
container_name: (cut)...
environment: (cut)...
volumes: (cut)...
ports: (cut)...
restart: (cut)...
```

---

These 6 options describe the settings for the container:

Firstly we have `image`, which is set as a URL pointing us to the image used for this container, in our case it is set as `lscr.io/linuxserver/jellyfin:latest`. We probably don't want to pin it to latest which `:latest` is telling us. Lets go find the latest version! Ill look on
https://github.com/linuxserver/docker-jellyfin/pkgs/container/jellyfin which tells us the latest version is `10.10.7`

Next of we have `container_name`, which is a friendly name that makes the container appears under when we run `docker ps`, if we didn't set this it would use a random name like `friendly-panda-76`.

The 3rd option is environment, which is one of the more used options when setting up docker compose files, these options set enviroment variables inside the containers.

```yaml
environment:
	# This tells the container to display logs with time in new zealand
	# You can find one for your country here:
	# https://en.wikipedia.org/wiki/List_of_tz_database_time_zones
	- TZ=Pacific/Auckand

	# These two options tell us what user to run the container as
	# You can find the numbers to fill in here with the id command:
	# The uid will go in pgid and the gid will go in pgid
    - PUID=1000
	- PGID=1000

	# This is specific for jellyfin, you can read more about this on
	# the linuxserver page
	# I set this to my local ip.
    - JELLYFIN_PublishedServerUrl=http://10.0.0.39 #optional
```

The next section is important to not forget - `volumes`, these allow you to persist data between container restarts, since normally any data gets wiped on reboot. For jellyfin we set several volumes for the different types of media we have.

```yaml
volumes:
	# this is binding a path to one in the container.
	# The first section before the : is on your computer
	# the next is inside the container
	- /media/movies:/data/moves:
	- /media/tvseries:/data/tvseries:

	# i make this readable so you can configure settings in jellyfin.
	- /docker/jellyfin/config:/config
```

The final two options are probably the most simple, `ports` and `restart`. The ports option is a way to access services inside the container. for us with jellyfin that means we can access the website. if the web server was running on port `5566` in the container and we want to access it on port `80` on the main computer we can do:

```yaml
ports:
	- 80:5566
```

Restart is telling us what the container should do if the `host` (Your computer) restarts, for instance, should the container turn on when he computer starts as well, or not.

So now that we know all of this we can assemble the compose file for our jellyfin server!

```yaml
---
services:
  jellyfin:
    image: lscr.io/linuxserver/jellyfin:10.10.7
    container_name: jellyfin
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Pacific/Auckland
      - JELLYFIN_PublishedServerUrl=http://10.0.0.39
    volumes:
      - /docker/jellyfin/config:/config
      - /media/tvseries:/data/tvshows
      - /media/movies:/data/movies
    ports:
      - 80:8096
    restart: unless-stopped
```

> Thanks, this looks more readble now. Still a little lost.

Thats nothing new, it took me a _months_ to understand how it worked, but if you use it enough you'll understand.

---

In this part we are going to use this config we made in the previous step to run a jellyfin instance, lets go!

Firstly we need to create all the volumes we specified in the compose file, this is to avoid permission stuff you _really_ don't want to deal with:

```
mkdir -p /docker/jellyfin/config /media{tvseries,movies}
```

Now we need to create a `docker-compose.yml` file with the config we made in a previous step. This can be anywhere on your system, but im going to put in in `/docker/jellyfin/docker-compose.yml`.

```
cd /docker/jellyfin
nano docker-compose.yml
(paste)...
(ctrl x + y and enter to save!)
```

> And now what?

We need to run it, of course!

This is easily done with the magic of docker compose, which we spent all this time setting up!

```bash
docker compose up
```

Yeah results!

```
root@jellyfin-school:/docker/jellyfin# docker compose up
[+] Running 1/1
 ✔ jellyfin Pulled                                                                                                                                                                                     3.0s
[+] Running 2/2
 ✔ Network jellyfin_default  Created                                                                                                                                                                   0.1s
 ✔ Container jellyfin        Created                                                                                                                                                                   0.1s
Attaching to jellyfin
jellyfin  | [migrations] started
jellyfin  | [migrations] no migrations found
jellyfin  | ───────────────────────────────────────
(cut)...
```

> And what? Some magic lines of scrolling text? Where's my _media server_!.

No we need to check that it works first, silly. Remember that port you set earlier? time to use that!

for me i'm pretty sure that was `10.0.0.39`. Lets try it.

![[Jellfin Website Success.png]]

And... Success! We see jellyfin's setup wizard. Now that we know it works, we can set it up to run in the background so we dont have to have the ssh connenction runing the whole time. Firstly, press `ctrl + c` to shutdown the container then run `docker compose up -d` to run it in the background. You can shut the terminal now if you wish.

Lets move on to setting up jellyfin itself.

---

The first few steps are obvious ie:

1. Select you language
2. Make an admin account, for me `admin admin` as im not using this for production.

The next step is setting up your media libraries, you click the big add media library button and it shows up with this

![[Media Server library setup.png]]

> Looks self explanatory enough

Agreed. We have 2 librarys that we setup in the compose file, so we have to do this twice. For movies we slect movies and...

![[Movies oh its complicated now.png]]

> oh.

Wow... that's a lot of options that suddenly appeared, lets look through this. it looks like there is a lot of blank options that need filling in, so we'll look at those first. The download language is most likely for subtitles and movie images, and the location is so we get close download servers. Not too difficult.

Scrolling down a bit further we see options for metadata servers which we can mostly ignore, since the defaults should mostly be good enough for our usecase. Near the bottom of the options are a few checkboxes, one of them being `Save artwork into media folders`. I’ll turn this on as the default option is to just dump the artwork right next to where the media downloaded. You can choose to enable any of the other options, but im not going to.

Hit that OK button!
![[whoops forgot the folders.png]]

> Hah you forgot something.

umm. yeah i did, i got so focussed on options i forgot to set the most important bit! Where you store your media. Scrolling way up to the top we see a little plus button i forgot to click.

![[the folders button jellyfin.png]]

Clicking on that big forgettable plus button we see a popup of the paths where our media is stored, and jellyfin is smart enough to not show anything except what we need.

![[the path selection jellyfin.png]]

> How clever.

We are doing movies this time so we'll click on `/data/movies`. Hit that OK button!

![[movie library setup.png]]

Woohoo! now we need to setup tvshows, basically the same except for tvshows where we select movies.

![[tvshows media library jellyfin.png]]

> That easy?

yeah that easy, thanks jellyfin. onwards to the next step!

---

![[jellyfin metadata settings.png]]

> _Really jellyfin? Couldn't you have let us set those before we setup the library's?_

Im with you there, nothing crazy for these settings

![[remote access settings.png]]

These options are perfect on defaults, since you probably don't want other people to access your server remotely with an unsecured connection. If that is your goal, look into reverse proxies like [caddy](https://caddyserver.com/) to secure your connection.

![[finished setup jellyfin.png]]
_Woo were done!_

Nice and simple, time to finish this off and watch some movies!

---

![[jellyfin login page.png]]
_Hope you didnt forget your login._

Wait!

> what?

We forgot something

> Like what?

Media! it is a media server after all.

> Well... How we gonna do that?

With the magic of `scp` of course, fire up that terminal again!

As an example of a movie im going to use blenders
[Big Buck Bunny](https://peach.blender.org/download/), make sure you have a movie or other video you want to watch available on your computer. For me i have big buck bunny stored at
`~/media/video/moves/bbb_sunflower_2160p_60fps_normal.mp4`, we will use that for the following command:

```bash
scp ~/media/video/movies/bbb_sunflower_2160p_60fps_normal.mp4 root@10.0.0.39:/media/movies
```

> Looks kinda like the ssh command.

Your right! the `:/data/movies` is telling us we want to copy the video to the server at the path `/data/movies`, lets run this and see if it worked.

```
➜  ~  scp ~/media/video/movies/bbb_sunflower_2160p_60fps_normal.mp4 root@10.0.0.39:/media/movies
bbb_sunflower_2160p_60fps_nor 100%  642MB  28.6MB/s   00:22
```

z
looks like it did, lets check back on the website to see if it showed up.

![[jellyfin no movies.png]]

> Nothing?

annoyingly so, we need to manually update jellyfins library so that it realizes it exists, its not too difficult as its a single button push.

Click on the hamburger menu on the top left, and you should see a bunch of options show up

![[dashboard menu jellyfin.png]]

Head to the dashboard page, there should be a button near the top named `Scan All Libraries`, click that and wait for it to finish loading. When that finished you can go back to the main menu by clicking on the jellyfin logo in the top left. You should be greeted by the beutiful sight of your movie!

![[jellyfin with a movie showing up.png]]

> How exciting!

Most definitally, lets click on it and give it a watch.

![[movie playing jellyfin.png]]
And it runs!
