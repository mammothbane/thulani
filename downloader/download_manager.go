package downloader

import (
	"time"

	"github.com/bwmarrin/discordgo"
)

type DlMessage int

const (
	Clear DlMessage = iota
	Pause
	Play
)

type playBundle struct {
	data <-chan []byte
	conn *discordgo.VoiceConnection
}

type DownloadManager struct {
	session *discordgo.Session
	guildID string
	voiceID string
	dls     chan *downloader

	PlayState chan DlMessage
}

func NewManager(s *discordgo.Session, guildID string, voiceChanID string) *DownloadManager {
	dm := &DownloadManager{
		session:   s,
		dls:       make(chan *downloader),
		PlayState: make(chan DlMessage),
		guildID:   guildID,
		voiceID:   voiceChanID,
	}

	go dm.playFromQueue()

	return dm
}

func (m *DownloadManager) playFromQueue() {
	for dl := range m.dls {
		conn, err := m.session.ChannelVoiceJoin(m.guildID, m.voiceID, false, false)
		if err != nil {
			log.Errorf("unable to connect to the voice channel: %q", err)
			time.Sleep(1 * time.Second)
			break
		}

		out, _ := dl.Start()

		playState := Play
		conn.Speaking(true)

		cleanup := func() {
			conn.Speaking(false)
			conn.Disconnect()
		}

	inner:
		for {
			switch playState {
			case Clear:
				for {
					select {
					case <-m.PlayState:
					default:
					}
				}
				break inner
			case Pause:
				playState = <-m.PlayState
			case Play:
				select { // first check if we have a state update message coming in
				case playState = <-m.PlayState:
				case elem, ok := <-out:
					if !ok {
						break inner
					}

					conn.OpusSend <- elem
				}
			}
		}
		cleanup()
	}
}

func (m *DownloadManager) Enqueue(url string, startTime, duration time.Duration) error {
	dl, err := newDownload(url, startTime, duration)
	if err != nil {
		return err
	}
	m.dls <- dl
	return nil
}
