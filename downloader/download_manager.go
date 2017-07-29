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

	PlayState      chan DlMessage
	playStateChan  chan DlMessage
	proxyStateChan chan DlMessage

	proxyChan chan playBundle
}

const proxyBufSize = 512

func NewManager(s *discordgo.Session, guildID string, voiceChanID string) *DownloadManager {
	dm := &DownloadManager{
		session:   s,
		dls:       make(chan *downloader),
		PlayState: make(chan DlMessage),
		guildID:   guildID,
		voiceID:   voiceChanID,

		playStateChan: make(chan DlMessage),

		proxyStateChan: make(chan DlMessage),
		proxyChan:      make(chan playBundle),
	}

	go dm.teeStateMessages()
	go dm.proxyOpusPackets()
	go dm.playFromQueue()

	return dm
}

func (m *DownloadManager) teeStateMessages() {
	for msg := range m.PlayState {
		m.playStateChan <- msg
		m.proxyStateChan <- msg
	}
}

func (m *DownloadManager) proxyOpusPackets() {
loop:
	for bundle := range m.proxyChan {
		playState := Play
		bundle.conn.Speaking(true)

		cleanup := func() {
			bundle.conn.Speaking(false)
			bundle.conn.Disconnect()
		}

		for {
			switch playState {
			case Clear:
				for {
					select {
					case <-m.proxyChan:
					default:
					}
				}
				cleanup()
				continue loop
			case Pause:
				playState = <-m.proxyStateChan
			case Play:
				select { // first check if we have a state update message coming in
				case playState = <-m.proxyStateChan:
				case bundle.conn.OpusSend <- <-bundle.data:
				}
			}
		}
		cleanup()
	}
}

func (m *DownloadManager) playFromQueue() {
	for dl := range m.dls {
		ch, err := m.session.ChannelVoiceJoin(m.guildID, m.voiceID, false, false)
		if err != nil {
			log.Errorf("unable to connect to the voice channel: %q", err)
			time.Sleep(1 * time.Second)
			break
		}

		out, done := dl.Start()
		m.proxyChan <- playBundle{
			data: out,
			conn: ch,
		}
		<-done
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
