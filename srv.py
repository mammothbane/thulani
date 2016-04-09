import discord
import asyncio
import logging
import re
from urllib.parse import urlsplit

logging.basicConfig(level=logging.INFO)

client = discord.Client()
client.queue = asyncio.Queue(maxsize=5)
client.current_player = None

@client.event
@asyncio.coroutine
def on_ready():
    logging.info('Logged in as\n\t{}\n\t{}'.format(client.user.name, client.user.id))


bot_name = ''
server_name = ''
reg = re.compile(r'^(?:!|\/){} (.*)$'.format(bot_name))
main_player_id = 0

@client.event
@asyncio.coroutine
def on_message(message):
    global reg
    global main_player_id

    logging.debug('received message %s' % message)
    if message.channel.is_private:
        return

    if message.server.name != server_name:
        logging.info('wrong server %s' % message.server.name)
        return

    match = reg.search(message.content)
    if not match:
        logging.info('match failed')
        return

    command = match.group(1).split()[0]
    if command == 'stop' and int(message.author.id) == main_player_id:
        if client.current_player and client.current_player.is_playing():
            client.current_player.stop()
            return

    if command == 'pause' and int(message.author.id) == main_player_id:
        if client.current_player and client.current_player.is_playing():
            client.current_player.pause()
            return        

    if command == 'resume' and int(message.author.id) == main_player_id:
        if client.current_player and not client.current_player.is_playing():
            client.current_player.resume()
            return        


    url = urlsplit(command, scheme='https')
    if not (url.netloc and (url.path or (url.path is '/watch' and not url.query))):
        yield from client.send_message(message.channel, 'format your commands right. fuck you.', tts=message.tts)
        return

    url = url.geturl()
    logging.info(url)

    if not client.is_voice_connected():
        logging.info('connecting')
        voice_chan = discord.utils.find(lambda x: x.name == 'General' and x.type is discord.ChannelType.voice,
                                        message.server.channels)
        if not voice_chan:
            logging.error('no voice channel')

        yield from client.join_voice_channel(voice_chan)

    if client.current_player and client.current_player.is_playing():
        client.current_player.stop()

    client.current_player = yield from client.voice.create_ytdl_player(url)
    client.current_player.start()


    # enqueue_video(url, client, message.channel)

def enqueue_video(url, client, channel):
    if not client.is_voice_connected():
        logging.info
        voice_chan = discord.utils.find(lambda x: x.name == 'General' and x.type is discord.ChannelType.voice, 
                                        channel.server.channels)
        yield from client.join_voice_channel(voice_chan)

    if not client.is_voice_connected():
        yield from client.send_message(channel, 'go fuck yourself. voice isn\'t working.', tts=True)
        return

    if client.queue.full():
        yield from client.send_message(channel, 'fuck you. wait for the other videos.', tts=True)
        return

    elem = yield from client.voice.create_ytdl_player(url)
    client.queue.put(elem)


def run_video(client, loop):
    logging.info('looping')
    if client.current_player and client.current_player.is_playing():
        return

    client.current_player = yield from client.queue.get()
    client.current_player.start()

    loop.call_later(1, run_video, client, loop)


loop = asyncio.get_event_loop()
loop.call_soon(run_video, client, loop)

username = ''
password = ''
client.run(username, password)
