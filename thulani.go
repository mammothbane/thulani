package thulani

import (
	"fmt"
	"net/url"
	"os"
	"os/signal"
	"regexp"
	"strings"
	"syscall"

	"github.com/bwmarrin/discordgo"
)

var config *Config
var regex *regexp.Regexp

func Run(conf *Config) {
	config = conf
	regex = regexp.MustCompile("(?i)^[!/]" + conf.Trigger + " (.*)")

	dg, err := discordgo.New("Bot " + conf.Token)
	handle(err)

	dg.AddHandler(onReady)
	dg.AddHandler(onMessage)
	dg.Open()

	joined := false

	for !joined {
		for _, v := range dg.State.Guilds {
			if v.Name == conf.Server {
				joined = true
				break
			}
		}

		if !joined {
			fmt.Println("Please input the token for whatever the fuck.")
			var response string
			fmt.Scanln(&response)
		}
	}

	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
	<-sc

	dg.Close()
}

func onReady(s *discordgo.Session, m *discordgo.Ready) {
	log.Infof("Logged in as %v (%v)", m.User.Username, m.User.ID)
}

func onMessage(s *discordgo.Session, m *discordgo.MessageCreate) {
	if m.Author.ID == s.State.User.ID {
		return
	}

	log.Debugf("got message %q", m.Content)

	ctx, err := newCtx(s, m)
	if err != nil {
		log.Errorf("error constructing message context: %q", err)
	}

	if ctx.Channel.IsPrivate {
		log.Infof("Ignoring private message")
		return
	}

	if ctx.Channel.GuildID != config.Server {
		log.Infof("Wrong guild. Ignoring.")
		return
	}

	_ = func() bool {
		for _, v := range ctx.Member.Roles {
			if v == config.OpRole {
				return true
			}
		}
		log.Infof("User %v not authorized.", m.Author.Username)

		ctx.sendMessage("fuck you. you're not allowed to do that.", m.Tts)
		return false
	}

	//fmap := map[string]func(){
	//	"sup": func() {},
	//}[ctx.Command]

	switch ctx.Command {
	case "skip":
		break

	case "die":
		break

	default:
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
			break
		}

		// TODO: play audio
	}
}
