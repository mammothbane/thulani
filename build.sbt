lazy val root = (project in file("."))

libraryDependencies ++= Seq(
	"com.github.austinv11" % "Discord4J" % "2.4.6",
	"org.slf4j" % "slf4j-simple" % "1.7.9",
	"org.yaml" % "snakeyaml" % "1.17",
	"org.scala-lang" % "scala-reflect" % "2.11.8"
)

resolvers ++= Seq(
	"jcenter-bintray" at "http://jcenter.bintray.com",
	"jitpack.io" at "https://jitpack.io"
)

scalacOptions ++= Seq("-deprecation", "-feature", "-language:implicitConversions")

scalaVersion := "2.11.8"
