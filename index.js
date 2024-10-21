import { Client, GatewayIntentBits, EmbedBuilder, Events, SlashCommandBuilder, REST, Routes } from 'discord.js';
import { createClient } from 'redis';

const redisClient = createClient({url: process.env.REDIS_URL});
redisClient.on('error', err => console.log('Redis Client Error', err));
await redisClient.connect();

const client = new Client({	intents: [
		GatewayIntentBits.Guilds,
		GatewayIntentBits.GuildMessages,
		GatewayIntentBits.MessageContent,
		GatewayIntentBits.GuildMembers,
	]});

const rest = new REST().setToken(process.env.DISCORD_TOKEN);

const WHITELIST_CHANNELS = [
  '967074642076516367',
  '1074813849712214127',
  '1059235902771187793',
  '999826882461700258',
  '1042114977928065076',
  '1000506173713297431',
  '1008859522309312602',
  '1005528820075466964',
  '1028658894333038672',
  '979999280339247125',
  '1050847825723916382',
  '1041479219097645148',

  // Own testing server
  '1272161749155446794'
];

const MEDALS = [
  "🥇",
  "🥈",
  "🥉",
];

const commands = [
  {
    data: new SlashCommandBuilder()
      .setName("paw")
      .setDescription("Paws!")
      .addSubcommand(subcommand => {
        return subcommand.setName("daily").setDescription("Claim your daily paw!")
      })
  },
];

client.on(Events.InteractionCreate, async (interaction) => {
  if (!interaction.isChatInputCommand()) return;
  const { commandName, options } = interaction;
  const subcommand = options.getSubcommand();

  if (subcommand == "daily") {
    if (await redisClient.get(`daily-${interaction.member.user.id}`) == "true") {
      return await interaction.reply({ephemeral: true, content: "You've already claimed your daily paw!"})
    }
    await redisClient.set(`daily-${interaction.member.user.id}`, "true", {
      EX: Math.ceil(Date.now() / 60 / 60 / 24) * 24 * 60 * 60 - Date.now()
    });
    const paws = await redisClient.incr(`${interaction.member.user.id}`);
    await interaction.reply({content: `You claimed your daily paw, and now hold onto ${paws} paws!`})
  }
});

const randomTimeBetween = (min, max) =>
  Math.round(Math.random() * (max - min) + min);

client.on('ready', async () => {
  console.log(`Logged in as ${client.user.tag}!`);
  try {
    console.log('Started refreshing application (/) commands.');
    let cmds = commands.map((c) => c.data.toJSON());
    await rest.put(
        Routes.applicationGuildCommands(process.env.DISCORD_CLIENT_ID, process.env.DISCORD_GUILD),
        { body: cmds },
    );

    console.log('Successfully reloaded application (/) commands.');
  } catch (error) {
    console.error(error);
  }
});

client.on(Events.MessageCreate, async (message) => {
  if (!message.author || message.author.bot) return;
  if (!WHITELIST_CHANNELS.includes(message.channelId)) return;

  if (await redisClient.get('cooldown') == "true") {
    if (message.content === "🐶") {
      const [lastChannel, pawId] = (await redisClient.get('lastpaw'))?.split('-') || [];

      if (lastChannel != message.channelId) return;

      message.channel.messages.fetch(pawId)
        .then((m) => m?.delete())
        .catch(() => {});
      await message.delete().catch(() => {});

      const paws = await redisClient.incr(message.author.id);
      await redisClient.del('lastpaw');

      const claimedMsg = new EmbedBuilder()
        .setTitle("🐶 paw claimed 🐶")
        .setDescription(`<@${message.author.id}> has claimed a paw and now holds onto ${paws}`)
        .setColor(0x11111c)
        .setThumbnail(message.author.avatarURL())
        .setFooter({ text: 'This message will self distruct in 10 seconds.' });
      const claimedReply = await message.channel.send({embeds: [claimedMsg]});
      setTimeout(async () => { await claimedReply.delete() }, 10000);
    }
    return;

  } else {
    const [lastChannel, pawId] = (await redisClient.get('lastpaw'))?.split('-') || [];
    if (lastChannel != undefined || pawId != undefined) {

      message.guild.channels.cache.get(lastChannel).messages.fetch(pawId)
        .then((m) => m?.delete())
        .catch(() => { });
    }
  }

  // if (Math.random() > 0.3) return;
  const reply = await message.channel.send("🐶");

  const cooldown = randomTimeBetween(3 * 60, 20 * 60);
  await redisClient.set('cooldown', "true", { EX: cooldown });
  await redisClient.set('lastpaw', `${message.channelId}-${reply.id}`);
});

client.login(process.env.DISCORD_TOKEN);
