package downloader

// DownloadManager handles a download for a particular song.
type DownloadManager struct {
	Url string

	dlChans []<-chan []byte
}

func NewDownload(url string) DownloadManager {
	return DownloadManager{
		Url:     url,
		dlChans: []<-chan []byte{},
	}
}
