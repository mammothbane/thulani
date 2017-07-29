package thulani

import (
	"encoding/json"
	"net/url"
	"os"
	"strconv"
	"sync"

	"github.com/bwmarrin/discordgo"
	"github.com/op/go-logging"
)

const help = `wew lad. you should know these commands already.

Usage: ` + "`!thulani [command]`" + `

commands:
**help**:				print this help message
**[url]**:			   a url with media that thulani can play. queued up to play after everything that's already waiting.
**list, queue**:	list items in the queue, as well as the currently-playing item.
**pause**:			pause sound.
**resume**:		 resume sound.
**die**:				 empty the queue and stop playing.
**skip**:			   skip the current item.
`

var log = logging.MustGetLogger("thulani")

type Config struct {
	Trigger        string `json:"trigger"`
	QueueSize      uint   `json:"queue_size"`
	AdminID        uint   `json:"admin_id"`
	OpRoleID       uint   `json:"op_role_id"`
	GuildID        uint   `json:"guild_id"`
	VoiceChannelID uint   `json:"voice_channel_id"`
	Token          string `json:"token"`
	ClientID       string `json:"client_id"`
	ClientSecret   string `json:"client_secret"`
}

func (c *Config) GuildStr() string {
	return strconv.Itoa(int(c.GuildID))
}

func (c *Config) VoiceChannelStr() string {
	return strconv.Itoa(int(c.VoiceChannelID))
}

func (c *Config) AdminStr() string {
	return strconv.Itoa(int(c.AdminID))
}

func (c *Config) OpRoleStr() string {
	return strconv.Itoa(int(c.OpRoleID))
}

func handle(err error) {
	if err != nil {
		log.Fatal(err)
	}
}

func LoadConfig(filename string) (*Config, error) {
	file, err := os.Open(filename)
	if err != nil {
		return nil, err
	}

	var conf Config
	err = json.NewDecoder(file).Decode(&conf)
	return &conf, err
}

const requestedPerms = discordgo.PermissionEmbedLinks |
	discordgo.PermissionReadMessages |
	discordgo.PermissionAddReactions |
	discordgo.PermissionSendMessages |
	discordgo.PermissionSendTTSMessages |
	discordgo.PermissionMentionEveryone |
	discordgo.PermissionUseExternalEmojis |
	discordgo.PermissionVoiceConnect |
	discordgo.PermissionVoiceSpeak |
	discordgo.PermissionChangeNickname |
	discordgo.PermissionVoiceUseVAD |
	discordgo.PermissionAttachFiles

var _oauthUrl string
var oauthOnce sync.Once

func oauthUrl() string {
	oauthOnce.Do(func() {
		oUrl, err := url.Parse("https://discordapp.com/api/oauth2/authorize")
		if err != nil {
			panic(err)
		}

		q := oUrl.Query()
		q.Add("scope", "bot")
		q.Add("permissions", strconv.Itoa(requestedPerms))
		q.Add("client_id", config.ClientID)
		oUrl.RawQuery = q.Encode()

		_oauthUrl = oUrl.String()
	})

	return _oauthUrl
}
