package thulani

import (
	"os"
	"os/signal"
	"syscall"

	"github.com/bwmarrin/discordgo"
)

func Run(conf *Config) {
	dg, err := discordgo.New("Bot " + conf.Token)
	handle(err)

	dg.AddHandler(onReady)
	dg.Open()

	sc := make(chan os.Signal, 1)
	signal.Notify(sc, syscall.SIGINT, syscall.SIGTERM, os.Interrupt, os.Kill)
	<-sc

	dg.Close()
}

func onReady(s *discordgo.Session, m *discordgo.Ready) {
	log.Debugf("Logged in as %v (%v)", m.User.Username, m.User.ID)
}
