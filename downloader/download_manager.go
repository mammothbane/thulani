package downloader

import "github.com/bwmarrin/discordgo"

type DownloadManager struct {
	session *discordgo.Session
}

func NewManager(s *discordgo.Session) *DownloadManager {

	return &DownloadManager{
		session: s,
	}
}
