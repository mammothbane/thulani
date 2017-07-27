package thulani

import (
	"strings"
	"sync"

	"github.com/bwmarrin/discordgo"
)

type messageCtx struct {
	sync.Mutex

	*discordgo.Session
	*discordgo.MessageCreate

	Command string
	Matched bool

	Channel *discordgo.Channel
	Member  *discordgo.Member
	Guild   *discordgo.Guild
}

func newCtx(s *discordgo.Session, m *discordgo.MessageCreate) (*messageCtx, error) {
	matches := regex.FindStringSubmatch(m.Content)
	command := ""

	if len(matches) != 0 {
		command = strings.Split(matches[1], " ")[0]
	}

	channel, err := s.State.Channel(m.ChannelID)
	if err != nil {
		return nil, err
	}

	var (
		wg         sync.WaitGroup
		guild      *discordgo.Guild
		member     *discordgo.Member
		gErr, mErr error
	)

	wg.Add(2)
	go func() {
		guild, gErr = s.State.Guild(channel.GuildID)
		defer wg.Done()
	}()

	go func() {
		member, mErr = s.GuildMember(channel.GuildID, m.Author.ID)
		defer wg.Done()
	}()
	wg.Wait()

	if gErr != nil {
		return nil, gErr
	}

	if mErr != nil {
		return nil, mErr
	}

	return &messageCtx{
		Session:       s,
		MessageCreate: m,

		Command: command,
		Matched: len(matches) == 0,

		Channel: channel,
		Guild:   guild,
		Member:  member,
	}, nil
}

func (ctx *messageCtx) sendMessage(str string, tts bool) {
	if !ctx.Tts {
		ctx.ChannelMessageSend(ctx.ChannelID, str)
		return
	}

	ctx.ChannelMessageSendTTS(ctx.ChannelID, str)
}
