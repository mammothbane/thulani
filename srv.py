import discord
import logging
import re
import yaml
from asyncio import coroutine, Queue, ensure_future as async, QueueEmpty, sleep
from urllib.parse import urlsplit
from functools import partial

logging.basicConfig(level=logging.INFO)

logger = logging.getLogger('yt-bot')
logger.setLevel(logging.DEBUG)

with open('config.yml') as f:
    config = yaml.load(f)

client = discord.Client()
client.queue = Queue(maxsize=config.get('queue_size', 0))
client.current_player = None

reg = re.compile(r'^(?:!|\/){} (.*)$'.format(config['trigger']))


@client.event
@coroutine
def on_ready():
    logger.info('Logged in as {} ({})'.format(client.user.name, client.user.id))


@client.event
@coroutine
def on_message(message):
    logger.debug('received message \'{}\' from {}#{}, ({})'.format(message.content,
                                                                   message.server,
                                                                   message.channel,
                                                                   'private' if message.channel.is_private else 'public'))
    
    if message.channel.is_private:
        logger.debug('ignoring private message.')
        return

    comp = message.server.id if type(config['server']) is int else message.server.name

    if comp != config['server']:
        logger.debug('message from wrong server ({})'.format(comp))
        return

    match = reg.search(message.content)
    if not match:
        logger.debug('no match.')
        return

    commands = match.group(1).split()
    command = commands[0]
    author_id = int(message.author.id)

    cmd_map = {
        'skip': stop_player,
        'die': stop_client,
        'sudoku': stop_client,
        'pause': pause,
        'resume': resume,
        'list': partial(list_queued, channel=message.channel),
        'queue': partial(list_queued, channel=message.channel),
    }

    if command in cmd_map:
        if author_id == config['admin'] or config['op_role'] in [role.name for role in message.author.roles]:
            logger.info('running command \'{}\''.format(command))
            async(cmd_map[command](client))        
            return

        logger.info('unauthorized command \'{}\' from member \'{}\' ({})'.format(command, message.author.name, message.author.id))
        async(client.send_message(message.channel, 'fuck you. you\'re not allowed to do that.', tts=message.tts))
        return

    url = urlsplit(command, scheme='https')
    if not (url.netloc and (url.path or (url.path is '/watch' and not url.query))):
        logger.info('syntax error: invalid url \'{}\''.format(command))
        async(client.send_message(message.channel, 'format your commands right. fuck you.', tts=message.tts))
        return

    url = url.geturl()
    logger.debug('playing video from url \'{}\''.format(url))

    async(enqueue_video((url, message), client, message.channel))


@coroutine
def pause(client):
    if not client.current_player:
        return

    client.current_player.pause()


@coroutine
def resume(client):
    if not client.current_player:
        return

    client.current_player.resume()


@coroutine
def stop_player(client):
    if not client.current_player:
        return

    client.current_player.stop()
    client.current_player = None


@coroutine
def stop_client(client):
    if not client.current_player:
        return

    while True:
        try:
            client.queue.get_nowait()
        except QueueEmpty:
            break

    yield from async(stop_player(client))


@coroutine
def enqueue_video(pair, client, channel):
    global config
    
    yield from async(connect_voice(client))

    if not client.is_voice_connected():
        async(client.send_message(channel, 'go fuck yourself. voice isn\'t working.', tts=True))
        return

    if client.queue.full():
        async(client.send_message(channel, 'fuck you. wait for the other videos.', tts=True))
        return

    async(client.queue.put(pair))


@coroutine
def connect_voice(client):
    if not client.is_voice_connected():
        server = discord.utils.find(lambda x: x.name == config['server'], client.servers)
        voice_chan = discord.utils.find(lambda x: x.name == config['voice_channel'] and x.type is discord.ChannelType.voice, 
                                        server.channels)
        yield from client.join_voice_channel(voice_chan)


@coroutine
def list_queued(client, channel):
    s = ''
    count = 0

    def list_resp(s, count):
        if len(s.strip()) == 0:
            s = 'Queue empty\n'

        slots = config.get('queue_size', 0) 
        if slots is not 0:
            s += '{} slots remaining in the queue.'.format(slots - count)

        async(client.send_message(channel, s.strip()))


    if client.current_player and not client.current_player.is_done():
        s += '**{}**: {}\n\n'.format('playing' if client.current_player.is_playing() else 'paused', client.current_player.title)
    
    pairs = []
    while True:
        try:
            pairs.append(client.queue.get_nowait())

        except QueueEmpty:
            break

    if len(pairs) == 0:
        list_resp(s, count)
        return

    yield from async(connect_voice(client))

    if not client.is_voice_connected():
        async(client.send_message(channel, 'go fuck yourself. couldn\'t check stored videos', tts=True))
        logger.error('unable to connect to voice!')
        for pair in pairs:
            yield from client.queue.put(pair)
        return

    for (url, msg) in pairs:
        if len(msg.embeds) == 0:
            logger.debug('got non-embedded link. creating player to find title.')
            player = yield from client.voice.create_ytdl_player(url)
            name = player.title
        else:
            name = msg.embeds[0].get('title', None)

        s += '{}\n'.format(name if name and name != '' else '(Unknown)')
        count += 1

        yield from client.queue.put((url, msg))

    list_resp(s, count)


@coroutine
def run_video(client):
    vid_logger = logger.getChild('video_scheduler')
    vid_logger.debug('entering run_video')

    (url, _) = yield from client.queue.get()
    yield from connect_voice(client)

    if not client.is_voice_connected():
        raise Exception('unable to connect to voice!')
    else:
        vid_logger.info('voice reconnected')

    vid_logger.debug('starting playback')
    client.current_player = yield from client.voice.create_ytdl_player(url)
    client.current_player.start()
    vid_logger.debug('playback started')

    has_slept = False
    while True:
        if not client.current_player or client.current_player.is_done():
            break

        if not has_slept:
            vid_logger.debug('sleeping')
            has_slept = True

        yield from sleep(2)
    
    if has_slept:
        vid_logger.debug('awoken')

    async(run_video(client))


async(run_video(client))
client.run(config['username'], config['password'])
