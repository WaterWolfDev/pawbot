import { Client, GatewayIntentBits, EmbedBuilder, Events, SlashCommandBuilder, REST, Routes, userMention } from 'discord.js';
import { createClient } from 'redis';
import { openSync, closeSync, unlinkSync } from 'fs';
import * as http from 'node:http'

const redisClient = createClient({url: process.env.REDIS_URL});
redisClient.on('error', err => console.log('Redis Client Error', err));
await redisClient.connect();

const pubSubRedisClient = createClient({url: process.env.REDIS_URL});
pubSubRedisClient.on('error', err => console.log('Redis Client Error', err));
await pubSubRedisClient.connect();

console.log('subscribing to event')
pubSubRedisClient.subscribe('__keyspace@0__:cooldown', (message) => {
  console.log(message);
});

const client = new Client({
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.MessageContent,
    GatewayIntentBits.GuildMembers,
  ],
  allowedMentions: { parse: ['users', 'roles'], repliedUser: true }
});

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
      .addSubcommand(subcommand => {
        return subcommand.setName("top").setDescription("View the pawdium.")
      })
      .addSubcommand(subcommand => {
        return subcommand.setName("give").setDescription("Give your paws to another fur.")
        .addUserOption(option => {
          return option.setName("who").setDescription("Who do you want to donate paws to?").setRequired(true)
        })
        .addIntegerOption(option => {
          return option.setName("count").setDescription("How many paws?").setRequired(true)
        })
      })
      .addSubcommand(subcommand => {
        return subcommand.setName("steal").setDescription("If you're lucky, you might be able to do it...")
          .addUserOption(option => {
            return option.setName("who").setDescription("Who do you want steal paws from?").setRequired(true)
          })
          .addIntegerOption(option => {
            return option.setName("count").setDescription("How many paws?").setRequired(true)
          })
      })
      .addSubcommand(subcommand => {
        return subcommand.setName("gamble").setDescription("Test your odds")
          .addIntegerOption(option => {
            return option.setName("count").setDescription("How many paws?").setRequired(true)
          })
      })
  },
];

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

  if (subcommand == "top") {
    const count = Math.min(Math.max(3, 0 || 10), 20);

    const paws = {};
    for await (const key of redisClient.scanIterator({MATCH: "[1-9]*"})) {
      paws[key] = await redisClient.get(key);
    }
    const pawsSorted = Object.fromEntries(
        Object.entries(paws).sort(([,a],[,b]) => b-a)
    );
    const pawsArray = Object.keys(pawsSorted);
    const ownIndex = pawsArray.findIndex(
      (item) => item === interaction.member.user.id
    );

    let description = `🐶 **${Object.values(pawsSorted)
      .reduce((a, b) => Number(a) + Number(b), 0)
      .toLocaleString()}**\n`;
    description += ` 🧑‍🌾 **${pawsArray.length}**\n\n`;
    description += `📈 **Ranks** 💪\n`;

    for (const user of Object.keys(pawsSorted).slice(0, Math.max(3, count - 1))) {
      const paws = pawsSorted[user];
      const place = pawsArray.indexOf(user);
      description += `\` ${
        MEDALS[place] || `${(place + 1).toString().padStart(2, ' ')} `
      } \` **<@${user}>** - ${paws.toLocaleString()} paws\n`;
    }

    if (ownIndex >= count) {
      description += `\` ... \` *${ownIndex - 9}* other farmers\n`;
      description += `\` ${(ownIndex + 1).toString().padStart(2, ' ')} \` **<@${
        interaction.member.user.id
      }>** - ${
        pawsSorted[interaction.member.user.id]
      } paw${pawsSorted[interaction.member.user.id] === 1 ? '' : 's'}`;
    } else if (count > 3) {
      const user = pawsArray[count - 1];
      const userPaws = pawsSorted[user];
      description += `\` ${count.toString().padStart(2, ' ')}  \` **${ userMention(user) }** - ${paws[user]} paws\n`;
    }

    await interaction.reply({
      embeds: [
        new EmbedBuilder()
          .setTitle(`🏆 Leaderboard​ 👑`)
          .setDescription(description)
      ],
    });
  }

  if (subcommand == "give") {
    const who = interaction.options.getUser('who');
    const count = interaction.options.getInteger('count');
    if (who.id == interaction.user.id) {
      await interaction.reply({ content: 'Get outta here', ephemeral: true });
      return;
    }
    const userpaws = await redisClient.get(interaction.user.id) || 0;
    if (count > userpaws) {
      await interaction.reply({ content: `You can only give as many paws as you have! (up to ${userpaws})`, ephemeral: true });
    }
    if (count < 1) {
      await interaction.reply({ content: 'You need to send at least one paw.', ephemeral: true });
    }

    await redisClient.decrBy(interaction.user.id, count);
    await redisClient.incrBy(who.id, count);

    await interaction.reply({ content: `You gave ${count} paw${
      count === 1 ? '' : 's'
    } to ${userMention(who.id)}, how nice of you!` });
  }

  if (subcommand == "steal") {
    const who = interaction.options.getUser('who');
    const count = interaction.options.getInteger('count');

    if (who.id == interaction.user.id) {
      await interaction.reply({ content: "You can't steal from yourself!", ephemeral: true });
      return;
    }

    if (await redisClient.get(`steal-${interaction.user.id}`) == "true") {
      await interaction.reply({ content: "The fuzz is hot on your tail, lay low for a while.", ephemeral: true });
      return;
    }

    if (await redisClient.get(interaction.user.id) < count) {
      await interaction.reply({ content: 'You can only steal as many paws as you have!', ephemeral: true });
      return;
    }

    if (await redisClient.get(who.id) < count) {
      await interaction.reply({ content: "That user doesn't have enough paws!", ephemeral: true });
      return;
    }

    if (count < 1) {
      await interaction.reply({ content: 'You must steal at least one paw!', ephemeral: true });
      return;
    }

    if (count > 10) {
      await interaction.reply({ content: 'You can only steal 10 or less paws at a time!', ephemeral: true });
      return;
    }

    let winner = who.id;
    let loser = interaction.user.id;
    if (Math.random() < 0.33) {
      winner = interaction.user.id;
      loser = who.id;
    }
    const success = winner == interaction.user.id;

    await redisClient.incrBy(winner, count);
    await redisClient.decrBy(loser, count);

    let newUserPaws = await redisClient.get(interaction.user.id);

    await interaction.reply({
      embeds: [
        new EmbedBuilder().setTitle('🧤 🐶 🧤')
          .setDescription(`Your thievery ${
            success ? 'paid off' : 'sucked'
          }, you ${success ? 'stole' : 'gave'} ${count} paw${
            count === 1 ? '' : 's'
          } ${success ? 'from' : 'to'} ${userMention(who.id)}, ${
            success ? 'giving you a total of' : 'leaving you with'
          } ${newUserPaws} paw${
            newUserPaws === 1 ? '' : 's'
          }. ${'🐶'.repeat(newUserPaws)} ${
            success ? '📈' : '📉'
          }`)
          .setColor(0x11111c),
      ],
    });

    await redisClient.set(`steal-${interaction.user.id}`, "true", {
      EX: randomTimeBetween(3 * 60, 10 * 60)
    });
  }

  if (subcommand == "gamble") {
    const count = interaction.options.getInteger('count');

    if (await redisClient.get(`gamble-${interaction.user.id}`) == "true") {
      await interaction.reply({ content: "⛔🐶 Gambling addiction is a serious problem. Regulations require a wait. Try again later...", ephemeral: true });
      return;
    }

    const currentCount = redisClient.get(interaction.user.id) || 0;
    if (count > currentCount) {
      await interaction.reply({ content: 'You can only gamble as many paws as you have! (Up to 10)', ephemeral: true });
      return;
    }

    if (count > 10 || count < 1) {
      await interaction.reply({ content: 'You can only gamble between 1 and 10 paws.', ephemeral: true });
      return;
    }

    await redisClient.set(`gamble-${interaction.user.id}`, "true", { EX: randomTimeBetween(2 * 60, 5 * 60) });

    const won = Math.random() > 0.5;
    let newCount = 0;
    if (won) {
      newCount = await redisClient.incrBy(interaction.user.id, count);
    } else {
      newCount = await redisClient.decrBy(interaction.user.id, count);
    }

    await interaction.reply({
      embeds: [
        new EmbedBuilder()
          .setTitle(`🎲 🐶 🎲`)
          .setDescription(`Your gambling ${won ? 'paid off' : 'sucked'}, you ${
            won ? 'won' : 'lost'
          } ${count} paw${count === 1 ? '' : 's'}, ${
            won ? 'giving you' : 'leaving you with'
          } a total of ${newCount} paw${
            newCount === 1 ? '' : 's'
          }. ${'🐶'.repeat(newCount)} ${
            won ? '📈' : '📉'
          }`)
          .setColor(0x11111c),
      ],
    });
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
      await message.delete().catch((e) => {
        console.error(`failed to delete paw message ${e}`)
      });

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
    if (lastChannel != undefined && pawId != undefined) {
      message.guild.channels.cache.get(lastChannel).messages.fetch(pawId)
        .then((m) => m?.delete())
        .catch((e) => { console.error(`failed to delete paw message ${e}`) });
      await redisClient.del('lastpaw');
    }
  }

  if (Math.random() > 0.3) return;
  const reply = await message.channel.send("🐶");

  const cooldown = randomTimeBetween(3 * 60, 20 * 60);
  await redisClient.set('cooldown', "true", { EX: cooldown });
  await redisClient.set('lastpaw', `${message.channelId}-${reply.id}`);
});

process.on('SIGTERM', function onSigterm () {
  console.info('Got SIGTERM. Graceful shutdown start', new Date().toISOString())
  process.exit()
})

client.login(process.env.DISCORD_TOKEN);


// Allows for fetching of paws (not cooldowns) in case we need
// to restore in another location.
const host = '0.0.0.0';
const port = 8000;
const requestListener = async function (req, res) {
  if (req.url === '/api/paws') {
    const paws = {};
    for await (const key of redisClient.scanIterator({MATCH: "[1-9]*"})) {
      paws[key] = parseInt(await redisClient.get(key), 10);
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify(paws));
  } else if (req.url === '/') {
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end('<h1>Welcome to the Pawbot API</h1><p>Visit <a href="/api/paws">/api/paws</a> to get paw data.</p>');
  } else if (req.url === '/healthz') {
    res.writeHead(client.isReady() ? 200 : 503, { 'Content-Type': 'text/plain' });
    res.end(`${client.isReady()}`);
  } else {
    res.writeHead(404, { 'Content-Type': 'text/html' });
    res.end('');
  }
};

const server = http.createServer(requestListener);
server.listen(port, host, () => {
    console.log(`Server is running on http://${host}:${port}`);
});
