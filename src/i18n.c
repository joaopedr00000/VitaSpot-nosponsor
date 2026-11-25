/* Native UI message catalogue. Only app-owned strings are translated. */
#include "i18n.h"
#include "prefs.h"
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <limits.h>
#include "vam_backend.h"
static const char *const messages[][4] = {
    {"Audius - Trending", "Audius - Em alta", "Audius - Tendencias", "Audius - Tendances"},
    {"Search Audius", "Buscar no Audius", "Buscar en Audius", "Rechercher sur Audius"},
    {"Search public Audius tracks.", "Busque faixas públicas do Audius.", "Busca pistas públicas de Audius.", "Recherchez les titres publics d’Audius."},
    {"Audius library is not available yet.", "A biblioteca do Audius ainda não está disponível.", "La biblioteca de Audius aún no está disponible.", "La bibliothèque Audius n’est pas encore disponible."},
    {"Music provided by Audius.", "Música fornecida pelo Audius.", "Música proporcionada por Audius.", "Musique fournie par Audius."},
    {"Free account: music provided by Audius.", "Conta Free: músicas fornecidas pelo Audius.", "Cuenta Free: música de Audius.", "Compte Free : musique fournie par Audius."},
    {"Choose a track from Audius.", "Escolha uma faixa do Audius.", "Elige una pista de Audius.", "Choisissez un titre Audius."},
    {"Not available in Audius mode.", "Indisponível no modo Audius.", "No disponible en modo Audius.", "Indisponible en mode Audius."},
    {"No more Audius tracks.", "Não há mais faixas do Audius.", "No hay más pistas de Audius.", "Plus de titres Audius."},
    {"Audius connection failed.", "Falha na conexão com o Audius.", "Falló la conexión con Audius.", "Connexion à Audius impossible."},
    {"Audius rate limit reached.", "Limite de solicitações do Audius atingido.", "Límite de solicitudes de Audius alcanzado.", "Limite de requêtes Audius atteinte."},
    {"Audius content unavailable.", "Conteúdo do Audius indisponível.", "Contenido de Audius no disponible.", "Contenu Audius indisponible."},
    {"Audius response too large.", "Resposta do Audius muito grande.", "Respuesta de Audius demasiado grande.", "Réponse Audius trop volumineuse."},
    {"Invalid Audius response.", "Resposta inválida do Audius.", "Respuesta de Audius no válida.", "Réponse Audius invalide."},
    {"Unsupported Audius audio format.", "Formato de áudio do Audius não suportado.", "Formato de audio de Audius no compatible.", "Format audio Audius non pris en charge."},
    {"Audius temporary file failed.", "Falha no arquivo temporário do Audius.", "Falló el archivo temporal de Audius.", "Échec du fichier temporaire Audius."},
    {"Audius track too large.", "Faixa do Audius muito grande.", "Pista de Audius demasiado grande.", "Titre Audius trop volumineux."},
    {"Audius audio output failed.", "Falha na saída de áudio do Audius.", "Falló la salida de audio de Audius.", "Échec de la sortie audio Audius."},
    {"Audius audio worker failed.", "Falha no processamento de áudio do Audius.", "Falló el procesamiento de audio de Audius.", "Échec du traitement audio Audius."},
    {"Tweaks", "Tweaks", "Tweaks", "Tweaks"},
    {"Ad blocker", "Bloqueador de anúncios", "Bloqueador de anuncios", "Bloqueur de publicités"},
    {"Enabled", "Ativado", "Activado", "Activé"},
    {"Disabled", "Desativado", "Desactivado", "Désactivé"},
    {"Changes apply immediately.", "As alterações são imediatas.", "Los cambios son inmediatos.", "Les modifications sont immédiates."},
    {"Ad blocker disabled.", "Bloqueador de anúncios desativado.", "Bloqueador de anuncios desactivado.", "Bloqueur de publicités désactivé."},
    {"Ad blocker enabled.", "Bloqueador de anúncios ativado.", "Bloqueador de anuncios activado.", "Bloqueur de publicités activé."},
    {"Could not save ad blocker setting.", "Não foi possível salvar a opção de bloqueio.", "No se pudo guardar el ajuste de bloqueo.", "Impossible d’enregistrer le réglage du blocage."},
    {"Could not write vam-ui.log", "Falha ao gravar vam-ui.log", "No se pudo escribir vam-ui.log", "Impossible d’écrire vam-ui.log"},
    {"Backend log: vam-diagnostic.log", "Log do backend: vam-diagnostic.log", "Registro: vam-diagnostic.log", "Journal : vam-diagnostic.log"},
    {"Backend log: vam.log (fallback)", "Log do backend: vam.log (alternativo)", "Registro: vam.log (alternativo)", "Journal : vam.log (secours)"},
    {"Backend log missing; send vam-ui.log", "Log do backend ausente; envie vam-ui.log", "Falta el registro; envía vam-ui.log", "Journal absent ; envoyez vam-ui.log"},
    {"Playback command failed.", "Falha no comando de reprodução.", "Falló el comando de reproducción.", "Échec de la commande de lecture."},
    {"Sign-in failed.", "Falha ao entrar.", "No se pudo iniciar sesión.", "Échec de connexion."},
    {"Spotify request failed.", "Falha na solicitação ao Spotify.", "Falló la solicitud a Spotify.", "Échec de la requête Spotify."},
    {"Search failed.", "Falha na busca.", "Falló la búsqueda.", "Échec de recherche."},
    {"Spotify is rate limiting requests. Try again soon.", "Spotify limitou as solicitações. Tente em breve.", "Spotify limitó las solicitudes. Reintenta pronto.", "Spotify limite les requêtes. Réessayez bientôt."},
    {"Spotify refused the request. Try logging in again.", "Spotify recusou a solicitação. Entre novamente.", "Spotify rechazó la solicitud. Inicia sesión de nuevo.", "Spotify a refusé la requête. Reconnectez-vous."},
    {"Connect discovery unavailable. Try QR pairing or PC login.", "Connect indisponível. Tente QR ou login no PC.", "Connect no disponible. Prueba QR o PC.", "Connect indisponible. Essayez le QR ou le PC."},
    {"Home", "Início", "Inicio", "Accueil"},
    {"Search", "Buscar", "Buscar", "Rechercher"},
    {"Library", "Biblioteca", "Biblioteca", "Bibliothèque"},
    {"Settings", "Configurações", "Ajustes", "Paramètres"},
    {"Language", "Idioma", "Idioma", "Langue"},
    {"Account", "Conta", "Cuenta", "Compte"},
    {"Screen", "Tela", "Pantalla", "Écran"},
    {"Audio", "Áudio", "Audio", "Audio"},
    {"About", "Sobre", "Acerca de", "À propos"},
    {"Song", "Música", "Canción", "Titre"},
    {"Songs", "Músicas", "Canciones", "Titres"},
    {"Episode", "Episódio", "Episodio", "Épisode"},
    {"Album", "Álbum", "Álbum", "Album"},
    {"Albums", "Álbuns", "Álbumes", "Albums"},
    {"Artist", "Artista", "Artista", "Artiste"},
    {"Artists", "Artistas", "Artistas", "Artistes"},
    {"Playlist", "Playlist", "Lista", "Playlist"},
    {"Playlists", "Playlists", "Listas", "Playlists"},
    {"Podcast", "Podcast", "Pódcast", "Podcast"},
    {"Couldn't load this", "Não foi possível carregar", "No se pudo cargar", "Chargement impossible"},
    {"Check your connection and try again.", "Verifique a conexão e tente novamente.", "Comprueba la conexión e inténtalo de nuevo.", "Vérifiez la connexion et réessayez."},
    {"Try again", "Tentar novamente", "Reintentar", "Réessayer"},
    {"Good morning", "Bom dia", "Buenos días", "Bonjour"},
    {"Good afternoon", "Boa tarde", "Buenas tardes", "Bonjour"},
    {"Good evening", "Boa noite", "Buenas noches", "Bonsoir"},
    {"What do you want to listen to?", "O que você quer ouvir?", "¿Qué quieres escuchar?", "Que voulez-vous écouter ?"},
    {"Play what you love", "Ouça o que você ama", "Escucha lo que te gusta", "Écoutez ce que vous aimez"},
    {"Search for songs, artists, albums and playlists.", "Busque músicas, artistas, álbuns e playlists.", "Busca canciones, artistas, álbumes y listas.", "Recherchez des titres, artistes, albums et playlists."},
    {"Recent searches", "Buscas recentes", "Búsquedas recientes", "Recherches récentes"},
    {"No results found for \"%s\"", "Nenhum resultado para \"%s\"", "Sin resultados para \"%s\"", "Aucun résultat pour \"%s\""},
    {"Check the spelling, or try different keywords.", "Confira a grafia ou tente outras palavras.", "Revisa la ortografía o prueba otras palabras.", "Vérifiez l’orthographe ou essayez d’autres mots."},
    {"All", "Tudo", "Todo", "Tout"},
    {"Nothing here yet", "Nada por aqui ainda", "Todavía no hay nada", "Rien pour le moment"},
    {"Your Library", "Sua biblioteca", "Tu biblioteca", "Votre bibliothèque"},
    {"Following", "Seguindo", "Siguiendo", "Abonné"},
    {"Follow", "Seguir", "Seguir", "Suivre"},
    {"Unfollow", "Deixar de seguir", "Dejar de seguir", "Se désabonner"},
    {"Queue", "Fila", "Cola", "File d’attente"},
    {"Your queue is empty", "Sua fila está vazia", "Tu cola está vacía", "Votre file est vide"},
    {"Pick something to play", "Escolha algo para ouvir", "Elige algo para escuchar", "Choisissez un titre"},
    {"ALBUM", "ÁLBUM", "ÁLBUM", "ALBUM"},
    {"PLAYLIST", "PLAYLIST", "LISTA", "PLAYLIST"},
    {"ARTIST", "ARTISTA", "ARTISTA", "ARTISTE"},
    {"YOUR LIBRARY", "SUA BIBLIOTECA", "TU BIBLIOTECA", "VOTRE BIBLIOTHÈQUE"},
    {"PLAYING FROM %s", "TOCANDO DE %s", "REPRODUCIENDO DE %s", "LECTURE DE %s"},
    {"PLAYING FROM", "TOCANDO DE", "REPRODUCIENDO DE", "LECTURE DE"},
    {"Playing on another device", "Tocando em outro dispositivo", "Reproduciendo en otro dispositivo", "Lecture sur un autre appareil"},
    {"VitaSpot is an unofficial Spotify client.", "VitaSpot é um cliente não oficial do Spotify.", "VitaSpot es un cliente no oficial de Spotify.", "VitaSpot est un client Spotify non officiel."},
    {"Remove from Liked Songs", "Remover das músicas curtidas", "Quitar de favoritos", "Retirer des titres aimés"},
    {"Add to Liked Songs", "Adicionar às músicas curtidas", "Añadir a favoritos", "Ajouter aux titres aimés"},
    {"Add to queue", "Adicionar à fila", "Añadir a la cola", "Ajouter à la file"},
    {"Go to album", "Ir para o álbum", "Ir al álbum", "Voir l’album"},
    {"Go to artist", "Ir para o artista", "Ir al artista", "Voir l’artiste"},
    {"Play", "Reproduzir", "Reproducir", "Lire"},
    {"Remove from Your Library", "Remover da biblioteca", "Quitar de tu biblioteca", "Retirer de la bibliothèque"},
    {"Save to Your Library", "Salvar na biblioteca", "Guardar en tu biblioteca", "Enregistrer dans la bibliothèque"},
    {"Added to queue", "Adicionado à fila", "Añadido a la cola", "Ajouté à la file"},
    {"Couldn't add to queue.", "Não foi possível adicionar à fila.", "No se pudo añadir a la cola.", "Ajout à la file impossible."},
    {"Unofficial client. Not affiliated with or endorsed by Spotify. Requires Spotify Premium.", "Cliente não oficial, sem vínculo com o Spotify. Reprodução sujeita à autorização.", "Cliente no oficial, sin vínculo con Spotify. Reproducción sujeta a autorización.", "Client non officiel, sans lien avec Spotify. Lecture soumise à autorisation."},
    {"Sign in to VitaSpot", "Entrar no VitaSpot", "Iniciar sesión en VitaSpot", "Connexion à VitaSpot"},
    {"Scan with your phone camera", "Escaneie com a câmera do celular", "Escanea con la cámara del móvil", "Scannez avec votre téléphone"},
    {"1. Open spotify.com/pair", "1. Abra spotify.com/pair", "1. Abre spotify.com/pair", "1. Ouvrez spotify.com/pair"},
    {"2. Enter this code", "2. Digite este código", "2. Introduce este código", "2. Saisissez ce code"},
    {"3. Approve on Spotify", "3. Autorize no Spotify", "3. Autoriza en Spotify", "3. Autorisez sur Spotify"},
    {"Waiting for approval...", "Aguardando autorização...", "Esperando autorización...", "En attente d’autorisation..."},
    {"Or use Spotify Connect on the same Wi-Fi.", "Ou use o Spotify Connect na mesma rede Wi-Fi.", "O usa Spotify Connect en la misma red Wi-Fi.", "Ou utilisez Spotify Connect sur le même Wi-Fi."},
    {"Starting up...", "Iniciando...", "Iniciando...", "Démarrage..."},
    {"Signing in...", "Entrando...", "Iniciando sesión...", "Connexion..."},
    {"Couldn't sign in", "Não foi possível entrar", "No se pudo iniciar sesión", "Connexion impossible"},
    {"Connect from the Spotify app", "Conecte pelo app Spotify", "Conecta desde la app Spotify", "Connectez depuis Spotify"},
    {"Tap the devices icon and choose \"%s\".", "Toque no ícone de dispositivos e escolha \"%s\".", "Pulsa el icono de dispositivos y elige \"%s\".", "Touchez l’icône des appareils et choisissez \"%s\"."},
    {"Connect this Vita and your phone to the same Wi-Fi.", "Conecte o Vita e o celular à mesma rede Wi-Fi.", "Conecta la Vita y el móvil a la misma red Wi-Fi.", "Connectez la Vita et le téléphone au même Wi-Fi."},
    {"Open Spotify on your phone or computer.", "Abra o Spotify no celular ou computador.", "Abre Spotify en el móvil u ordenador.", "Ouvrez Spotify sur votre téléphone ou ordinateur."},
    {"Or open %s and enter %s", "Ou abra %s e digite %s", "O abre %s e introduce %s", "Ou ouvrez %s et saisissez %s"},
    {"Or open %s", "Ou abra %s", "O abre %s", "Ou ouvrez %s"},
    {"Spotify Premium required", "Spotify Premium necessário", "Se requiere Spotify Premium", "Spotify Premium requis"},
    {"VitaSpot streams through Spotify Connect, which only works with Premium accounts.", "VitaSpot usa o Spotify Connect, que exige autorização para reproduzir.", "VitaSpot usa Spotify Connect, que requiere autorización para reproducir.", "VitaSpot utilise Spotify Connect, qui exige une autorisation de lecture."},
    {"Signed in as %s", "Conectado como %s", "Sesión iniciada como %s", "Connecté en tant que %s"},
    {"Log out", "Sair", "Cerrar sesión", "Déconnexion"},
    {"Cancel", "Cancelar", "Cancelar", "Annuler"},
    {"Log out of Spotify on this Vita?", "Sair do Spotify neste Vita?", "¿Cerrar sesión en esta Vita?", "Se déconnecter de Spotify sur cette Vita ?"},
    {"Couldn't log out. Try again.", "Não foi possível sair. Tente novamente.", "No se pudo cerrar sesión. Reintenta.", "Déconnexion impossible. Réessayez."},
    {"Never", "Nunca", "Nunca", "Jamais"},
    {"%d seconds", "%d segundos", "%d segundos", "%d secondes"},
    {"1 minute", "1 minuto", "1 minuto", "1 minute"},
    {"%d minutes", "%d minutos", "%d minutos", "%d minutes"},
    {"Screen off combo", "Atalho para apagar a tela", "Atajo para apagar pantalla", "Raccourci écran éteint"},
    {"Change", "Alterar", "Cambiar", "Modifier"},
    {"Hold two buttons...", "Segure dois botões...", "Mantén dos botones...", "Maintenez deux boutons..."},
    {"Dim screen after", "Escurecer tela após", "Atenuar pantalla tras", "Atténuer l’écran après"},
    {"Turn screen off after", "Apagar tela após", "Apagar pantalla tras", "Éteindre l’écran après"},
    {"Music keeps playing with the screen off. Press any button to turn it back on.", "A música continua com a tela apagada. Pressione um botão para ligá-la.", "La música sigue con la pantalla apagada. Pulsa un botón para encenderla.", "La musique continue écran éteint. Appuyez sur un bouton pour le rallumer."},
    {"High (320 kbps)", "Alta (320 kbps)", "Alta (320 kbps)", "Haute (320 kbps)"},
    {"Normal (160 kbps)", "Normal (160 kbps)", "Normal (160 kbps)", "Normale (160 kbps)"},
    {"Low (96 kbps)", "Baixa (96 kbps)", "Baja (96 kbps)", "Basse (96 kbps)"},
    {"Audio quality", "Qualidade de áudio", "Calidad de audio", "Qualité audio"},
    {"Audio cache", "Cache de áudio", "Caché de audio", "Cache audio"},
    {"Couldn't change the audio quality.", "Não foi possível alterar a qualidade.", "No se pudo cambiar la calidad.", "Qualité audio non modifiable."},
    {"Quality applies from the next song. Lower quality uses less data and starts faster.", "A qualidade muda na próxima faixa. Qualidade menor usa menos dados.", "La calidad cambia en la próxima canción. Menor calidad usa menos datos.", "La qualité change au titre suivant. Une qualité réduite économise les données."},
    {"Played songs are kept on the memory card. A new cache size applies after a restart.", "As faixas ficam no cartão de memória. O tamanho do cache muda ao reiniciar.", "Las canciones quedan en la tarjeta. El tamaño de caché cambia al reiniciar.", "Les titres restent sur la carte. La taille du cache change au redémarrage."},
    {"Version %s", "Versão %s", "Versión %s", "Version %s"},
    {"An unofficial Spotify client for the PS Vita. Not affiliated with Spotify.", "Cliente não oficial do Spotify para PS Vita, sem vínculo com o Spotify.", "Cliente no oficial de Spotify para PS Vita, sin vínculo con Spotify.", "Client Spotify non officiel pour PS Vita, sans lien avec Spotify."},
    {"Couldn't reach the player service.", "Não foi possível acessar o serviço de reprodução.", "No se pudo acceder al servicio de reproducción.", "Service de lecture inaccessible."},
    {"Spotify Premium is required to %s.", "Spotify Premium é necessário para %s.", "Se requiere Spotify Premium para %s.", "Spotify Premium est requis pour %s."},
    {"No connection. Couldn't %s.", "Sem conexão. Não foi possível %s.", "Sin conexión. No se pudo %s.", "Pas de connexion. Impossible de %s."},
    {"Couldn't %s.", "Não foi possível %s.", "No se pudo %s.", "Impossible de %s."},
    {"This song isn't available.", "Esta música não está disponível.", "Esta canción no está disponible.", "Ce titre est indisponible."},
    {"play this", "reproduzir", "reproducir", "lire ce titre"},
    {"change playback", "alterar a reprodução", "cambiar la reproducción", "modifier la lecture"},
    {"skip back", "voltar uma faixa", "retroceder", "revenir au titre précédent"},
    {"skip", "avançar", "saltar", "passer au titre suivant"},
    {"Couldn't update your library.", "Não foi possível atualizar a biblioteca.", "No se pudo actualizar tu biblioteca.", "Mise à jour de la bibliothèque impossible."},
    {"Removed from Liked Songs", "Removido das músicas curtidas", "Quitado de favoritos", "Retiré des titres aimés"},
    {"Added to Liked Songs", "Adicionado às músicas curtidas", "Añadido a favoritos", "Ajouté aux titres aimés"},
    {"Removed from Your Library", "Removido da biblioteca", "Quitado de tu biblioteca", "Retiré de la bibliothèque"},
    {"Saved to Your Library", "Salvo na biblioteca", "Guardado en tu biblioteca", "Enregistré dans la bibliothèque"},
    {"Something went wrong.", "Algo deu errado.", "Algo salió mal.", "Une erreur est survenue."},
    {"Recently played", "Tocadas recentemente", "Escuchado recientemente", "Écoutés récemment"},
    {"Your playlists", "Suas playlists", "Tus listas", "Vos playlists"},
    {"Saved albums", "Álbuns salvos", "Álbumes guardados", "Albums enregistrés"},
    {"Your artists", "Seus artistas", "Tus artistas", "Vos artistes"},
    {"Popular", "Populares", "Populares", "Populaires"},
    {"Singles and EPs", "Singles e EPs", "Sencillos y EP", "Singles et EP"},
    {"Appears on", "Participações", "Aparece en", "Apparaît sur"},
    {"Liked Songs", "Músicas curtidas", "Canciones favoritas", "Titres aimés"},
    {"Now playing", "Tocando agora", "Sonando ahora", "En cours"},
    {"Next up", "A seguir", "A continuación", "À suivre"},
    {"Wi-Fi is off. Connect to a network to sign in.", "Wi-Fi desligado. Conecte-se para entrar.", "Wi-Fi desactivado. Conéctate para iniciar sesión.", "Wi-Fi désactivé. Connectez-vous pour vous identifier."},
    {"Wi-Fi is off. Reconnect to continue.", "Wi-Fi desligado. Reconecte para continuar.", "Wi-Fi desactivado. Reconecta para continuar.", "Wi-Fi désactivé. Reconnectez-vous."},
    {"Not signed in to Spotify.", "Você não está conectado ao Spotify.", "No has iniciado sesión en Spotify.", "Non connecté à Spotify."},
    {"Not logged in to Spotify.", "Você não está conectado ao Spotify.", "No has iniciado sesión en Spotify.", "Non connecté à Spotify."},
    {"Not logged in.", "Você não está conectado.", "No has iniciado sesión.", "Non connecté."},
    {"Another Spotify device is controlling playback.", "Outro dispositivo controla a reprodução.", "Otro dispositivo controla la reproducción.", "Un autre appareil contrôle la lecture."},
    {"Nothing to play.", "Nada para reproduzir.", "Nada para reproducir.", "Rien à lire."},
    {"Spotify could not load this track. Check vam.log for the cause.", "O Spotify não carregou a faixa. Consulte vam.log.", "Spotify no pudo cargar el título. Consulta vam.log.", "Spotify n’a pas chargé le titre. Consultez vam.log."},
    {"Playback timed out. Check vam.log for authorization or network errors.", "Tempo de reprodução esgotado. Consulte vam.log.", "Tiempo de espera agotado. Consulta vam.log.", "Délai de lecture dépassé. Consultez vam.log."},
    {"Spotify wouldn't play these songs, so playback stopped. vam.log has the details.", "Spotify não reproduziu as faixas. Detalhes em vam.log.", "Spotify no reprodujo las canciones. Detalles en vam.log.", "Spotify n’a pas lu les titres. Détails dans vam.log."},
    {"Received Spotify credentials, signing in...", "Credenciais recebidas. Entrando...", "Credenciales recibidas. Iniciando sesión...", "Identifiants reçus. Connexion..."},
    {"Signing in to Spotify...", "Entrando no Spotify...", "Iniciando sesión en Spotify...", "Connexion à Spotify..."},
    {"Scan the QR code and enter the pairing code.", "Escaneie o QR e digite o código.", "Escanea el QR e introduce el código.", "Scannez le QR et saisissez le code."},
    {"Spotify did not authorize device pairing. Try Connect or PC login.", "Spotify não autorizou o pareamento. Tente Connect ou PC.", "Spotify no autorizó la vinculación. Prueba Connect o PC.", "Spotify a refusé l’association. Essayez Connect ou PC."},
    {"Spotify returned an unsupported pairing code.", "Spotify retornou um código incompatível.", "Spotify devolvió un código incompatible.", "Spotify a renvoyé un code incompatible."},
    {"Pairing code expired. Requesting a new code shortly.", "Código expirado. Solicitando outro em breve.", "Código caducado. Se solicitará otro pronto.", "Code expiré. Nouveau code sous peu."},
    {"Pairing declined. Try again shortly.", "Pareamento recusado. Tente novamente em breve.", "Vinculación rechazada. Reintenta pronto.", "Association refusée. Réessayez bientôt."},
    {"Spotify rejected the pairing request. Try Connect or PC login.", "Spotify recusou o pareamento. Tente Connect ou PC.", "Spotify rechazó la vinculación. Prueba Connect o PC.", "Spotify a rejeté l’association. Essayez Connect ou PC."},
    {"Pairing network request failed.", "Falha de rede no pareamento.", "Fallo de red al vincular.", "Échec réseau de l’association."},
    {"Could not initialize pairing network.", "Não foi possível iniciar o pareamento.", "No se pudo iniciar la vinculación.", "Impossible de démarrer l’association."},
    {"Could not read pairing response.", "Não foi possível ler a resposta.", "No se pudo leer la respuesta.", "Réponse d’association illisible."},
    {"Invalid pairing response.", "Resposta de pareamento inválida.", "Respuesta de vinculación inválida.", "Réponse d’association invalide."},
    {"Could not read pairing status.", "Não foi possível consultar o pareamento.", "No se pudo consultar la vinculación.", "État d’association illisible."},
    {"Invalid pairing status response.", "Estado de pareamento inválido.", "Estado de vinculación inválido.", "État d’association invalide."},
    {"Spotify did not return an access token.", "Spotify não retornou o token de acesso.", "Spotify no devolvió el token de acceso.", "Spotify n’a pas renvoyé de jeton."},
    {"Pairing approved, but Spotify session login failed.", "Pareamento aprovado, mas a sessão falhou.", "Vinculación aprobada, pero falló la sesión.", "Association approuvée, mais connexion échouée."},
    {"Pairing worker stopped.", "Processo de pareamento interrompido.", "Proceso de vinculación detenido.", "Processus d’association arrêté."},
    {"Open Spotify on your phone, tap the device picker and choose this device.", "Abra o Spotify no celular e escolha este dispositivo.", "Abre Spotify en el móvil y elige este dispositivo.", "Ouvrez Spotify sur votre téléphone et choisissez cet appareil."},
    {"Logged out. Open Spotify on your phone to sign in again.", "Sessão encerrada. Abra o Spotify no celular para entrar.", "Sesión cerrada. Abre Spotify en el móvil para entrar.", "Déconnecté. Ouvrez Spotify sur votre téléphone."},
    {"Reconnecting after suspend...", "Reconectando após suspensão...", "Reconectando tras suspensión...", "Reconnexion après suspension..."},
    {"The connection to Spotify was lost.", "A conexão com Spotify foi perdida.", "Se perdió la conexión con Spotify.", "Connexion à Spotify perdue."},
    {"Free account: browsing enabled; playback depends on Spotify authorization.", "Conta Free: navegação disponível; áudio depende de autorização.", "Cuenta Free: navegación disponible; audio sujeto a autorización.", "Compte Free : navigation disponible ; lecture soumise à autorisation."},
    {"Audio authorization failed. Check your account and vam.log.", "Falha na autorização do áudio. Verifique a conta e vam.log.", "Falló la autorización de audio. Revisa tu cuenta y vam.log.", "Autorisation audio refusée. Vérifiez le compte et vam.log."},
    {"Podcasts are not supported yet.", "Podcasts ainda não são suportados.", "Los pódcast aún no son compatibles.", "Podcasts pas encore pris en charge."},
    {"Spotify has no such item.", "Item não encontrado no Spotify.", "Spotify no tiene ese elemento.", "Élément introuvable sur Spotify."},
    {"Search isn't working right now.", "A busca não está funcionando agora.", "La búsqueda no funciona ahora.", "Recherche actuellement indisponible."},
    {"Up", "Cima", "Arriba", "Haut"},
    {"Down", "Baixo", "Abajo", "Bas"},
    {"Left", "Esquerda", "Izquierda", "Gauche"},
    {"Right", "Direita", "Derecha", "Droite"},
    {"Cross", "X", "Cruz", "Croix"},
    {"Circle", "Círculo", "Círculo", "Rond"},
    {"Triangle", "Triângulo", "Triángulo", "Triangle"},
    {"Square", "Quadrado", "Cuadrado", "Carré"},
    {"%d songs", "%d músicas", "%d canciones", "%d titres"},
    {"1 song", "1 música", "1 canción", "1 titre"},
    {"You", "Você", "Tú", "Vous"},
};
const char *tr(const char *key) {
    if (!key) return "";
    int lang = prefs_get()->language;
    if (lang <= 0 || lang >= LANG_COUNT) return key;
    for (unsigned i = 0; i < sizeof(messages)/sizeof(messages[0]); ++i)
        if (strcmp(messages[i][0], key) == 0) return messages[i][lang];
    return key;
}
const char *language_name(void) {
    static const char *const names[] = {"English", "Português (Brasil)", "Español", "Français"};
    int lang = prefs_get()->language;
    return names[(lang >= 0 && lang < LANG_COUNT) ? lang : 0];
}
void language_next(void) {
    prefs_get()->language = (prefs_get()->language + 1) % LANG_COUNT;
    prefs_changed();
}

/* Error details stay available in the English log. Only known diagnostic
 * prefixes are translated, never arbitrary song names or account names. */
const char *tr_message(const char *message) {
    if (!message) return "";
    const char *exact = tr(message);
    if (exact != message || prefs_get()->language == 0) return exact;
    static const struct { const char *prefix, *key; } patterns[] = {
        {"Sign-in failed:", "Sign-in failed."},
        {"Saved credentials were rejected:", "Sign-in failed."},
        {"Couldn't pause:", "Playback command failed."},
        {"Couldn't start playback:", "Playback command failed."},
        {"Playback command failed:", "Playback command failed."},
        {"Couldn't add to the queue:", "Couldn't add to queue."},
        {"Search failed:", "Search failed."},
        {"Couldn't sign the search request:", "Search failed."},
        {"Bad search request:", "Search failed."},
        {"Spotify request failed:", "Spotify request failed."},
        {"Bad playlist from Spotify:", "Spotify request failed."},
        {"Bad playlist list from Spotify:", "Spotify request failed."}
    };
    for (unsigned i = 0; i < sizeof(patterns)/sizeof(patterns[0]); ++i)
        if (strncmp(message, patterns[i].prefix, strlen(patterns[i].prefix)) == 0)
            return tr(patterns[i].key);
    return message;
}
const char *tr_subtitle(int kind, const char *value, char *buffer, size_t size, int detail) {
    if (!value || !*value || !buffer || size == 0 || prefs_get()->language == 0) return value;
    if (kind == VAM_KIND_ARTIST && strcmp(value, "Artist") == 0) return tr("Artist");
    if (!detail && (kind == VAM_KIND_ALBUM || kind == VAM_KIND_PLAYLIST || kind == VAM_KIND_COLLECTION)) {
        const char *label = kind == VAM_KIND_ALBUM ? "Album" : "Playlist";
        size_t len = strlen(label);
        if (strcmp(value, label) == 0) return tr(label);
        if (strncmp(value, label, len) == 0 && strncmp(value + len, " • ", strlen(" • ")) == 0) {
            const char *tail = value + len + strlen(" • ");
            if (kind == VAM_KIND_COLLECTION) {
                char *end;
                long n = strtol(tail, &end, 10);
                if (end != tail && n >= 0 && n <= INT_MAX && strcmp(end, " songs") == 0) {
                    char count[64];
                    if (n == 1) snprintf(count, sizeof(count), "%s", tr("1 song"));
                    else snprintf(count, sizeof(count), tr("%d songs"), (int)n);
                    snprintf(buffer, size, "%s • %s", tr(label), count);
                    return buffer;
                }
            }
            snprintf(buffer, size, "%s • %s", tr(label), tail);
            return buffer;
        }
    }
    if (detail && (kind == VAM_KIND_ALBUM || kind == VAM_KIND_PLAYLIST || kind == VAM_KIND_COLLECTION)) {
        const char *sep = NULL, *p = value;
        while ((p = strstr(p, " • ")) != NULL) { sep = p; p += strlen(" • "); }
        const char *tail = sep ? sep + strlen(" • ") : value;
        char *end;
        long n = strtol(tail, &end, 10);
        if (end != tail && n >= 0 && n <= INT_MAX &&
            (strcmp(end, " songs") == 0 || (n == 1 && strcmp(end, " song") == 0))) {
            char count[64];
            if (n == 1) snprintf(count, sizeof(count), "%s", tr("1 song"));
            else snprintf(count, sizeof(count), tr("%d songs"), (int)n);
            if (!sep) snprintf(buffer, size, "%s", count);
            else if (kind == VAM_KIND_COLLECTION && sep - value == 3 && strncmp(value, "You", 3) == 0)
                snprintf(buffer, size, "%s • %s", tr("You"), count);
            else snprintf(buffer, size, "%.*s • %s", (int)(sep - value), value, count);
            return buffer;
        }
    }
    return value;
}
