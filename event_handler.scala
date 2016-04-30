import sx.blah.discord._
import api.{EventSubscriber => Event}
import handle.impl.events.{ReadyEvent => Ready, _}

import org.slf4j._

object EventHandler {
	val logger = LoggerFactory getLogger EventHandler.getClass

	@Event
	def ready(event: Ready) = {
		val user = event.getClient.getOurUser
		logger info s"Logged in as ${user.getName} (${user.getID})"
	}

	@Event
	def message(event: MessageReceivedEvent): Unit = {
		val msg = event.getMessage
		logger info s"Received ${if (msg.getChannel.isPrivate) "private" else "public"} message '${msg.getContent}' from ${msg.getGuild.getName}#${msg.getChannel.getName}"+
			s"::${msg.getAuthor.getName} (${msg.getAuthor.getID})"

		if (msg.getChannel.isPrivate) {
			logger debug "Ignoring private message."
			return
		}

		if (msg.getGuild.getID != (Config \ "server": Int)) {
			logger debug s"Message from wrong server (${msg.getGuild.getName})"
			return
		}

	}
}
