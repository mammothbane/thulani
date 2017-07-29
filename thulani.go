package thulani

import (
	"net/url"
	"os"
	"os/signal"
	"regexp"
	"strings"
	"syscall"

	"github.com/bwmarrin/discordgo"
	"github.com/mammothbane/thulani-go/downloader"
)

var config *Config
var regex *regexp.Regexp
var manager *downloader.DownloadManager

func Run(conf *Config) {
	//defer profile.Start(profile.ProfilePath("."), profile.BlockProfile).Stop()

	config = conf
	regex = regexp.MustCompile(`(?i)^[!/]` + conf.Trigger + " (.*)")

	dg, err := discordgo.New("Bot " + conf.Token)
	handle(err)

	dg.AddHandler(onReady)
	dg.AddHandler(onMessage)
	dg.AddHandler(onGuildCreate)
	dg.Open()

	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
	<-sc

	dg.Close()
}

func onReady(s *discordgo.Session, m *discordgo.Ready) {
	log.Infof("Logged in as %v (%v)", m.User.Username, m.User.ID)

	s.UpdateStatus(0, "literally nothing")

	joined := false
	for _, v := range m.Guilds {
		if v.ID != config.GuildStr() {
			joined = true
			break
		}
	}

	if !joined {
		log.Warningf("Server in config not available! Click here to enable thulani on your server: %v", oauthUrl())
	}
	manager = downloader.NewManager(s, config.GuildStr(), config.VoiceChannelStr())
}

func onGuildCreate(s *discordgo.Session, m *discordgo.GuildCreate) {
	member, err := s.GuildMember(m.Guild.ID, s.State.User.ID)
	if err != nil {
		log.Warningf("joined guild %v but was unable to get member id: %q", m.Name, err)
		log.Notice("please reconnect to guild: %v", oauthUrl())
		s.GuildLeave(m.Guild.ID)
		return
	}
	log.Infof("joined guild %v", m.Name)

	perms := 0

	//log.Debugf("listing roles for %v", m.Name)
	for _, role := range m.Roles {
		//log.Debugf("%q (%v)", role.Name, role.ID)
		for _, mRole := range member.Roles {
			if role.ID == mRole {
				perms |= role.Permissions

				log.Infof("discovered own role: %v (%v)", role.Name, role.ID)
			}
		}
	}

	if perms&requestedPerms != requestedPerms {
		log.Errorf("server didn't grant us the desired permissions.")
		s.GuildLeave(m.Guild.ID)
		log.Warningf("Don't disable any permissions or thulani will be a little sponge man! Click here to die: %v", oauthUrl())
		return
	}

	err = s.GuildMemberNickname(m.Guild.ID, "@me", "newlani")
	if err != nil {
		log.Warningf("unable to update nickname: %q", err)
	}
}

func onMessage(s *discordgo.Session, m *discordgo.MessageCreate) {
	if m.Author.ID == s.State.User.ID {
		return
	}

	log.Debugf("got message %q", m.Content)

	ctx, err := newCtx(s, m)
	if err != nil {
		log.Errorf("error constructing message context: %q", err)
		return
	}

	for _, v := range extraMemes {
		v(ctx)
	}

	if !ctx.Matched {
		log.Infof("Message didn't match. Ignoring.")
		return
	}

	if ctx.Channel.IsPrivate {
		log.Infof("Ignoring private message")
		return
	}

	if ctx.Guild.ID != config.GuildStr() {
		log.Infof("Wrong guild. Ignoring.")
		return
	}

	fn, ok := cmdMap[strings.ToLower(ctx.Command)]
	if ok {
		log.Debugf("message matched a known command: %q", strings.ToLower(ctx.Command))

		authorized := ctx.Author.ID == config.AdminStr()

		if !authorized {
			authorMember, err := ctx.GuildMember(ctx.Guild.ID, ctx.Author.ID)
			if err != nil {
				log.Errorf("unable to get guild member for id %q", ctx.Author.Username)
				ctx.sendMessage("who the fuck are you?", true)
				return
			}

			for _, v := range authorMember.Roles {
				if v == config.OpRoleStr() {
					authorized = true
				}
			}
		}

		if !authorized {
			log.Infof("User %v not authorized.", m.Author.Username)
			ctx.sendMessage("fuck you. you're not allowed to do that.", m.Tts)
			return
		}

		log.Debugf("user was authorized for %q. executing.", strings.ToLower(ctx.Command))
		fn(ctx)
		return
	}

	// it's not a command we know; we're looking for a url
	target, err := url.Parse(ctx.Command)
	if err != nil {
		log.Errorf("Url parse failed: %q", err)
		ctx.sendMessage("format your commands right. fuck you.", m.Tts)
		return
	}

	if target.Path == "" || (target.Path == "/watch" && len(target.Query()) == 0) {
		log.Warningf("Bad url format: %q", ctx.Command)
		ctx.sendMessage("that\nis\na\nbad\nurl", m.Tts)
		return
	}

	if strings.Contains(target.Hostname(), "imgur") {
		log.Infof("Ignoring imgur link.")

		if m.Author.Username == "boomshticky" {
			ctx.sendMessage("fuck you conway", true)
		} else {
			ctx.sendMessage("NO IMGUR", m.Tts)
		}
	}

	if err := manager.Enqueue(target.String(), 0, 0); err != nil {
		log.Errorf("unable to enqueue video: %q", err)
		ctx.sendMessage("you fucked up the video.", ctx.Tts)
		return
	}
	log.Infof("started playing from: %q", target.String())
}
