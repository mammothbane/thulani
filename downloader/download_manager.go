package downloader

import (
	"time"

	"github.com/bwmarrin/discordgo"
)

type DlMessage int

const (
	Clear DlMessage = iota
	Pause
	Resume
)

type connUpdate int

const (
	attach connUpdate = iota
	detach
)

type DownloadManager struct {
	conn    *discordgo.VoiceConnection
	session *discordgo.Session
	guildID string
	voiceID string
	dls     chan *downloader

	PlayState      chan DlMessage
	playStateChan  chan DlMessage
	proxyStateChan chan DlMessage

	connUpdate chan connUpdate
	proxyChan  chan []byte
}

const proxyBufSize = 512

func NewManager(s *discordgo.Session, guildID string, voiceChanID string) *DownloadManager {
	dm := &DownloadManager{
		session:    s,
		dls:        make(chan *downloader),
		connUpdate: make(chan connUpdate, 1),
		PlayState:  make(chan DlMessage),
		guildID:    guildID,
		voiceID:    voiceChanID,

		playStateChan: make(chan DlMessage),

		proxyStateChan: make(chan DlMessage),
		proxyChan:      make(chan []byte, proxyBufSize),
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
	attachState := detach
	playState := Resume
	clear := false

loop:
	for {
		if playState == Pause {
			select {
			case playState = <-m.proxyStateChan:
				continue loop
			}
		}

		select { // first check if we have a state update message coming in
		case playState = <-m.proxyStateChan:
			continue loop
		default:
		}

		// if we're clearing, empty
		if clear {
			for {
				select {
				case <-m.proxyChan:
				case playState = <-m.proxyStateChan:
					continue loop
				default:
				}
			}
		}

		select {
		case upd := <-m.connUpdate:
			attachState = upd
		default:
		}

		if attachState == attach {
			select {
			case attachState = <-m.connUpdate:
			case m.conn.OpusSend <- <-m.proxyChan:
			}
		} else {
			select {
			case attachState = <-m.connUpdate:
			}
		}
	}
}

func (m *DownloadManager) playFromQueue() {
	//timer := time.After(5*time.Second)

	for {
		select {
		case dl := <-m.dls:
			if m.conn == nil {
				ch, err := m.session.ChannelVoiceJoin(m.guildID, m.voiceID, false, false)
				if err != nil {
					log.Errorf("unable to connect to the voice channel: %q", err)
					time.Sleep(1 * time.Second)
					break
				}

				m.conn = ch
				m.connUpdate <- attach
			}

			m.conn.Speaking(true)
			// todo: figure out how to do disconnection checking
			dl.SendOn(m.proxyChan)
			<-dl.done
			m.conn.Speaking(false)
			//case <-timer:
			//	if m.conn != nil {
			//		m.connUpdate<-detach
			//		if err := m.conn.Disconnect(); err != nil {
			//			log.Errorf("disconnecting from voice connection: %q", err)
			//		}
			//		m.conn = nil
			//	}
		}
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
