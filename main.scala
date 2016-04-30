object Main {
	import org.slf4j._
	import sx.blah.discord._

	val logger = LoggerFactory getLogger Main.getClass

	def main(args: Array[String]) = {
		import api._

		val client = (new ClientBuilder withLogin (Config \ "username", Config \ "password")).login
		client.getDispatcher registerListener EventHandler
	}
}

object Config {
	import scala.io.Source
	import org.yaml.snakeyaml._
	import scala.collection._
	import scala.collection.JavaConverters._
	import java.util.{Map => JMap}

	lazy val yaml: Map[String, Any] = ((new Yaml) load (Source fromFile "config.yml").mkString).asInstanceOf[JMap[String, Any]].asScala

	import scala.reflect.runtime.universe._
	def \[T: TypeTag](item: String): T = (yaml get item) match {
		case Some(x: String) if typeOf[T] <:< typeOf[Int] => x.toInt.asInstanceOf[T]
		case Some(x) => x.asInstanceOf[T]
		case None => throw new IllegalStateException(s"Config had no value for '$item'.")
	}
}
