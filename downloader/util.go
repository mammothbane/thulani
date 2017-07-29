package downloader

import (
	"encoding/json"
	"io/ioutil"
	"net/url"
	"os/exec"
	"time"

	"github.com/mammothbane/thulani-go/wav"
	"github.com/op/go-logging"
)

var log = logging.MustGetLogger("downloader")

// responsible for decoding from youtube
type videoInfo struct {
	Title       string        `json:"fulltitle"`
	UrlStr      string        `json:"url"`
	DurationSec int           `json:"duration"`
	Url         *url.URL      `json:"-"`
	Duration    time.Duration `json:"-"`
}

func info(inUrl string) (*videoInfo, error) {
	dl := exec.Command("youtube-dl", "-f", "bestaudio", "-x", "-j", inUrl)

	outpipe, err := dl.StdoutPipe()
	if err != nil {
		return nil, err
	}

	errpipe, err := dl.StderrPipe()
	if err != nil {
		return nil, err
	}

	err = dl.Start()
	if err != nil {
		log.Errorf("starting youtube-dl failed")
		return nil, err
	}

	o, ierr := ioutil.ReadAll(outpipe)
	if ierr != nil {
		log.Errorf("unable to read from output pipe")
		return nil, err
	}

	e, ierr := ioutil.ReadAll(errpipe)
	if ierr != nil {
		log.Errorf("unable to read from error pipe")
		return nil, err
	}

	if err := dl.Wait(); err != nil {
		log.Errorf("error:\n%v", string(e))
		return nil, err
	}

	v := videoInfo{}
	if err := json.Unmarshal(o, &v); err != nil {
		return nil, err
	}

	v.Duration = time.Duration(v.DurationSec) * time.Second
	v.Url, err = url.Parse(v.UrlStr)

	//tgt, err := url.Parse(string(o))
	//out := tgt.Scheme + "://" + tgt.Host + tgt.Path + "?" + tgt.Query().Encode()
	return &v, err
}

type wavBundle struct {
	wav     *wav.Wav
	cleanup func()
}
