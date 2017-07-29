package thulani

import (
	"net/url"
	"os"
	"os/signal"
	"regexp"
	"strings"
	"syscall"

	"time"

	"math/rand"

	"github.com/bwmarrin/discordgo"
	"github.com/mammothbane/thulani-go/downloader"
)

var config *Config
var regex *regexp.Regexp

func Run(conf *Config) {
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
		if v.Name == config.Server {
			joined = true
			break
		}
	}

	if !joined {
		log.Warningf("Server in config not available! Click here to enable thulani on your server: %v", oauthUrl())
	}
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

	for _, role := range m.Roles {
		for _, mRole := range member.Roles {
			if role.ID == mRole {
				perms |= role.Permissions

				log.Infof("discovered role: %v (%v)", role.Name, role.ID)
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

	for _, v := range ctx.Guild.Channels {
		if v.Type == "voice" && v.Name == "General" {
			conn, err := ctx.ChannelVoiceJoin(ctx.Guild.ID, v.ID, false, false)
			if err != nil {
				log.Errorf("unable to join voice channel: %q", err)
				break
			}

			ch, err := downloader.Download("https://www.youtube.com/watch?v=_K13GJkGvDw", time.Duration(rand.Intn(10*60))*time.Second, 5*time.Second)
			if err != nil {
				log.Errorf("unable to download video: %q", err)
				break
			}

			conn.Speaking(true)
			go func() {
				defer conn.Speaking(false)

				for i := range ch {
					conn.OpusSend <- i
				}
			}()

			break
		}
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

	if ctx.Guild.Name != config.Server {
		log.Infof("Wrong guild. Ignoring.")
		return
	}

	fn, ok := cmdMap[strings.ToLower(ctx.Command)]
	if ok {
		authorized := false

		for _, role := range ctx.Guild.Roles {
			for _, v := range ctx.Member.Roles {
				if v != role.Name {
					continue
				}

				if role.Name == config.OpRole {
					authorized = true
				}
			}
		}

		if !authorized {
			log.Infof("User %v not authorized.", m.Author.Username)
			ctx.sendMessage("fuck you. you're not allowed to do that.", m.Tts)
			return
		}

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
}
