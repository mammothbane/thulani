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
	dls     chan *Downloader

	PlayState      chan DlMessage
	playStateChan  chan DlMessage
	proxyStateChan chan DlMessage

	connUpdate chan connUpdate
	proxyChan  chan []byte
}

const proxyBufSize = 512

func NewManager(s *discordgo.Session) *DownloadManager {
	dm := &DownloadManager{
		session:    s,
		dls:        make(chan *Downloader),
		connUpdate: make(chan connUpdate, 1),
		PlayState:  make(chan DlMessage),

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
			case upd := <-m.connUpdate:
				attachState = upd
			case m.conn.OpusSend <- <-m.proxyChan:
			}
		} else {
			select {
			case upd := <-m.connUpdate:
				attachState = upd
			}
		}
	}
}

func (m *DownloadManager) playFromQueue() {
loop:
	for {
		select {
		case dl := <-m.dls:
			dl.SendOn(m.proxyChan)
			select {
			case <-dl.done:
				continue loop

			case upd:=<-m.playStateChan:

			}

		}
	}
}

func (m *DownloadManager) Enqueue(url string, startTime, endTime time.Duration) error {
	dl, err := NewDownload(url, startTime, endTime)
	if err != nil {
		return err
	}
	m.dls <- dl
	return nil
}
