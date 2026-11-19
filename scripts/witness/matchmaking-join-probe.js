(async () => {
  const NS = 'mm-join-' + Date.now().toString(36);
  const url = 'http://127.0.0.1:8853/js/client/play.html?ci=1&ns=' + NS;
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const hostPage = await context.newPage();
  state.mmHost = hostPage;
  await hostPage.goto(url, { waitUntil: 'domcontentloaded', timeout: 50000 });
  await wait(25000);
  const hostHash = new URL(hostPage.url()).hash;
  const joinerPage = await context.newPage();
  state.mmJoiner = joinerPage;
  await joinerPage.goto(url, { waitUntil: 'domcontentloaded', timeout: 50000 });
  await wait(25000);
  const joinerHash = new URL(joinerPage.url()).hash;
  const result = { ns: NS, hostHash, joinerHash, joinedSameRoom: hostHash.startsWith('#match-') && joinerHash === hostHash };
  state.mmJoinResult = result;
  await joinerPage.close();
  await hostPage.close();
  return result;
})()
