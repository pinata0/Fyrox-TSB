// Compile against the installed SDK headers so Rust never guesses EOS struct layouts.
#define _CRT_RAND_S
#include <stdlib.h>
#include <string.h>
#include "eos_sdk.h"
#include "eos_p2p.h"
#include "eos_version.h"

typedef struct TsbEos {
    EOS_HPlatform platform;
    EOS_HConnect connect;
    EOS_HP2P p2p;
    EOS_ProductUserId user;
    EOS_EResult login_result;
    EOS_EResult nat_result;
    EOS_ENATType nat;
    int login_started;
    int login_done;
    int nat_done;
    int closing;
    EOS_P2P_SocketId socket;
    EOS_ProductUserId remote;
    EOS_NotificationId request_notify;
    EOS_NotificationId established_notify;
    EOS_NotificationId auth_notify;
    int network_type;
    int p2p_open;
} TsbEos;

void tsb_eos_close(TsbEos* ctx);
static void EOS_CALL auth_expiring(const EOS_Connect_AuthExpirationCallbackInfo* info);

int32_t tsb_eos_initialize(void) {
    EOS_InitializeOptions options = {0};
    options.ApiVersion = EOS_INITIALIZE_API_LATEST;
    options.ProductName = "TSB";
    options.ProductVersion = "0.1.0";
    return (int32_t)EOS_Initialize(&options);
}
void tsb_eos_shutdown(void) { EOS_Shutdown(); }
const char* tsb_eos_version(void) { return EOS_GetVersion(); }
const char* tsb_eos_error(int32_t code) { return EOS_EResult_ToString((EOS_EResult)code); }

TsbEos* tsb_eos_create(const char* product, const char* sandbox, const char* deployment,
    const char* client, const char* secret, int32_t force_relay, int32_t* result) {
    EOS_Platform_Options options = {0};
    EOS_P2P_SetRelayControlOptions relay = {0};
    EOS_Connect_AddNotifyAuthExpirationOptions auth = {0};
    TsbEos* ctx = (TsbEos*)calloc(1, sizeof(TsbEos));
    if (!ctx) { *result = (int32_t)EOS_UnexpectedError; return NULL; }
    options.ApiVersion = EOS_PLATFORM_OPTIONS_API_LATEST;
    options.ProductId = product;
    options.SandboxId = sandbox;
    options.DeploymentId = deployment;
    options.ClientCredentials.ClientId = client;
    options.ClientCredentials.ClientSecret = secret;
    options.Flags = EOS_PF_DISABLE_OVERLAY;
    options.TickBudgetInMilliseconds = 2;
    ctx->platform = EOS_Platform_Create(&options);
    if (!ctx->platform) { free(ctx); *result = (int32_t)EOS_InvalidParameters; return NULL; }
    ctx->connect = EOS_Platform_GetConnectInterface(ctx->platform);
    ctx->p2p = EOS_Platform_GetP2PInterface(ctx->platform);
    if (!ctx->connect || !ctx->p2p) {
        EOS_Platform_Release(ctx->platform); free(ctx);
        *result = (int32_t)EOS_UnexpectedError; return NULL;
    }
    relay.ApiVersion = EOS_P2P_SETRELAYCONTROL_API_LATEST;
    relay.RelayControl = force_relay ? EOS_RC_ForceRelays : EOS_RC_AllowRelays;
    *result = (int32_t)EOS_P2P_SetRelayControl(ctx->p2p, &relay);
    if (*result != (int32_t)EOS_Success) { EOS_Platform_Release(ctx->platform); free(ctx); return NULL; }
    auth.ApiVersion = EOS_CONNECT_ADDNOTIFYAUTHEXPIRATION_API_LATEST;
    ctx->auth_notify = EOS_Connect_AddNotifyAuthExpiration(ctx->connect, &auth, ctx, auth_expiring);
    if (!ctx->auth_notify) { EOS_Platform_Release(ctx->platform); free(ctx); *result = (int32_t)EOS_UnexpectedError; return NULL; }
    return ctx;
}

void tsb_eos_destroy(TsbEos* ctx) {
    if (!ctx) return;
    ctx->closing = 1;
    if (ctx->auth_notify) EOS_Connect_RemoveNotifyAuthExpiration(ctx->connect, ctx->auth_notify);
    tsb_eos_close(ctx);
    // Keep callback storage alive until the platform has released all pending work.
    EOS_Platform_Release(ctx->platform);
    free(ctx);
}
void tsb_eos_tick(TsbEos* ctx) { EOS_Platform_Tick(ctx->platform); }

static void EOS_CALL created_user(const EOS_Connect_CreateUserCallbackInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    if (ctx->closing) return;
    ctx->login_result = info->ResultCode;
    ctx->login_done = 1;
    if (info->ResultCode == EOS_Success) ctx->user = info->LocalUserId;
}
static void EOS_CALL logged_in(const EOS_Connect_LoginCallbackInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    if (ctx->closing) return;
    if (info->ResultCode == EOS_InvalidUser && info->ContinuanceToken) {
        EOS_Connect_CreateUserOptions options = {0};
        options.ApiVersion = EOS_CONNECT_CREATEUSER_API_LATEST;
        options.ContinuanceToken = info->ContinuanceToken;
        EOS_Connect_CreateUser(ctx->connect, &options, ctx, created_user);
        return;
    }
    ctx->login_result = info->ResultCode;
    ctx->login_done = 1;
    if (info->ResultCode == EOS_Success) ctx->user = info->LocalUserId;
}
static void EOS_CALL created_device(const EOS_Connect_CreateDeviceIdCallbackInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    EOS_Connect_Credentials credentials = {0};
    EOS_Connect_UserLoginInfo user_info = {0};
    EOS_Connect_LoginOptions options = {0};
    if (ctx->closing) return;
    if (info->ResultCode != EOS_Success && info->ResultCode != EOS_DuplicateNotAllowed) {
        ctx->login_result = info->ResultCode;
        ctx->login_done = 1;
        return;
    }
    credentials.ApiVersion = EOS_CONNECT_CREDENTIALS_API_LATEST;
    credentials.Type = EOS_ECT_DEVICEID_ACCESS_TOKEN;
    user_info.ApiVersion = EOS_CONNECT_USERLOGININFO_API_LATEST;
    user_info.DisplayName = "TSB player";
    options.ApiVersion = EOS_CONNECT_LOGIN_API_LATEST;
    options.Credentials = &credentials;
    options.UserLoginInfo = &user_info;
    EOS_Connect_Login(ctx->connect, &options, ctx, logged_in);
}
void tsb_eos_login(TsbEos* ctx) {
    EOS_Connect_CreateDeviceIdOptions options = {0};
    if (ctx->login_started || ctx->closing) return;
    ctx->login_started = 1;
    options.ApiVersion = EOS_CONNECT_CREATEDEVICEID_API_LATEST;
    options.DeviceModel = "Windows PC";
    EOS_Connect_CreateDeviceId(ctx->connect, &options, ctx, created_device);
}
static void EOS_CALL auth_expiring(const EOS_Connect_AuthExpirationCallbackInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    if (ctx->closing) return;
    ctx->login_started = 0;
    ctx->login_done = 0;
    tsb_eos_login(ctx);
}
// -1 is pending; all other values are EOS_EResult codes.
int32_t tsb_eos_login_result(TsbEos* ctx) { return ctx->login_done ? (int32_t)ctx->login_result : -1; }
int32_t tsb_eos_user(TsbEos* ctx, char* buffer, int32_t capacity) {
    if (!ctx->user) return (int32_t)EOS_InvalidUser;
    return (int32_t)EOS_ProductUserId_ToString(ctx->user, buffer, &capacity);
}
static void EOS_CALL queried_nat(const EOS_P2P_OnQueryNATTypeCompleteInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    if (ctx->closing) return;
    ctx->nat_result = info->ResultCode;
    ctx->nat = info->NATType;
    ctx->nat_done = 1;
}
void tsb_eos_query_nat(TsbEos* ctx) {
    EOS_P2P_QueryNATTypeOptions options = {0};
    options.ApiVersion = EOS_P2P_QUERYNATTYPE_API_LATEST;
    ctx->nat_done = 0;
    EOS_P2P_QueryNATType(ctx->p2p, &options, ctx, queried_nat);
}
int32_t tsb_eos_nat_result(TsbEos* ctx) { return ctx->nat_done ? (int32_t)ctx->nat_result : -1; }
int32_t tsb_eos_nat_type(TsbEos* ctx) { return (int32_t)ctx->nat; }

static void EOS_CALL connection_requested(const EOS_P2P_OnIncomingConnectionRequestInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    EOS_P2P_AcceptConnectionOptions options = {0};
    if (ctx->closing || !ctx->p2p_open || !info->SocketId ||
        strcmp(info->SocketId->SocketName, ctx->socket.SocketName) != 0 ||
        (ctx->remote && ctx->remote != info->RemoteUserId)) return;
    // Transport acceptance is not game authorization. Signed invitations are
    // still checked by the game before a player can occupy a slot.
    options.ApiVersion = EOS_P2P_ACCEPTCONNECTION_API_LATEST;
    options.LocalUserId = ctx->user;
    options.RemoteUserId = info->RemoteUserId;
    options.SocketId = &ctx->socket;
    EOS_P2P_AcceptConnection(ctx->p2p, &options);
}
static void EOS_CALL connection_established(const EOS_P2P_OnPeerConnectionEstablishedInfo* info) {
    TsbEos* ctx = (TsbEos*)info->ClientData;
    if (!ctx->closing && ctx->p2p_open) ctx->network_type = (int)info->NetworkType;
}
void tsb_eos_close(TsbEos* ctx) {
    EOS_P2P_CloseConnectionsOptions options = {0};
    if (ctx->request_notify) EOS_P2P_RemoveNotifyPeerConnectionRequest(ctx->p2p, ctx->request_notify);
    if (ctx->established_notify) EOS_P2P_RemoveNotifyPeerConnectionEstablished(ctx->p2p, ctx->established_notify);
    ctx->request_notify = ctx->established_notify = 0;
    if (ctx->p2p_open) {
        options.ApiVersion = EOS_P2P_CLOSECONNECTIONS_API_LATEST;
        options.LocalUserId = ctx->user;
        options.SocketId = &ctx->socket;
        EOS_P2P_CloseConnections(ctx->p2p, &options);
    }
    ctx->p2p_open = 0;
    ctx->network_type = 0;
}
int32_t tsb_eos_open(TsbEos* ctx, const char* socket_name, const char* remote) {
    EOS_P2P_AddNotifyPeerConnectionRequestOptions request = {0};
    EOS_P2P_AddNotifyPeerConnectionEstablishedOptions established = {0};
    EOS_P2P_SetPacketQueueSizeOptions queues = {0};
    tsb_eos_close(ctx);
    if (!ctx->user || strlen(socket_name) != 32) return (int32_t)EOS_InvalidParameters;
    ctx->remote = remote ? EOS_ProductUserId_FromString(remote) : NULL;
    if (remote && (!EOS_ProductUserId_IsValid(ctx->remote) || ctx->remote == ctx->user))
        return (int32_t)EOS_InvalidUser;
    ctx->socket.ApiVersion = EOS_P2P_SOCKETID_API_LATEST;
    memcpy(ctx->socket.SocketName, socket_name, 33);
    queues.ApiVersion = EOS_P2P_SETPACKETQUEUESIZE_API_LATEST;
    queues.IncomingPacketQueueMaxSizeBytes = 256 * 1024;
    queues.OutgoingPacketQueueMaxSizeBytes = 256 * 1024;
    EOS_P2P_SetPacketQueueSize(ctx->p2p, &queues);
    request.ApiVersion = EOS_P2P_ADDNOTIFYPEERCONNECTIONREQUEST_API_LATEST;
    request.LocalUserId = ctx->user;
    request.SocketId = &ctx->socket;
    established.ApiVersion = EOS_P2P_ADDNOTIFYPEERCONNECTIONESTABLISHED_API_LATEST;
    established.LocalUserId = ctx->user;
    established.SocketId = &ctx->socket;
    ctx->p2p_open = 1;
    ctx->request_notify = EOS_P2P_AddNotifyPeerConnectionRequest(ctx->p2p, &request, ctx, connection_requested);
    ctx->established_notify = EOS_P2P_AddNotifyPeerConnectionEstablished(ctx->p2p, &established, ctx, connection_established);
    if (!ctx->request_notify || !ctx->established_notify) {
        tsb_eos_close(ctx);
        return (int32_t)EOS_UnexpectedError;
    }
    return (int32_t)EOS_Success;
}
int32_t tsb_eos_send(TsbEos* ctx, const char* peer, const void* data, uint32_t length) {
    EOS_P2P_SendPacketOptions options = {0};
    if (!ctx->p2p_open || length > EOS_P2P_MAX_PACKET_SIZE) return (int32_t)EOS_InvalidParameters;
    options.ApiVersion = EOS_P2P_SENDPACKET_API_LATEST;
    options.LocalUserId = ctx->user;
    options.RemoteUserId = EOS_ProductUserId_FromString(peer);
    if (!EOS_ProductUserId_IsValid(options.RemoteUserId) ||
        (ctx->remote && ctx->remote != options.RemoteUserId)) return (int32_t)EOS_InvalidUser;
    options.SocketId = &ctx->socket;
    options.DataLengthBytes = length;
    options.Data = data;
    options.Reliability = EOS_PR_UnreliableUnordered;
    // Only the repeatedly sent HELLO can wait for NAT negotiation. Never queue
    // stale movement, snapshots, or voice behind a broken connection.
    options.bAllowDelayedDelivery = length >= 5 &&
        (((const unsigned char*)data)[4] == 1 || ((const unsigned char*)data)[4] == 17);
    return (int32_t)EOS_P2P_SendPacket(ctx->p2p, &options);
}
// -1 means no matching packet, -2 means retryable send backpressure.
int32_t tsb_eos_receive(TsbEos* ctx, char* peer, int32_t peer_capacity,
    void* data, uint32_t capacity, uint32_t* written) {
    EOS_P2P_ReceivePacketOptions options = {0};
    EOS_P2P_SocketId socket = {0};
    EOS_ProductUserId remote = NULL;
    uint8_t channel = 0;
    EOS_EResult result;
    if (!ctx->p2p_open) return -1;
    options.ApiVersion = EOS_P2P_RECEIVEPACKET_API_LATEST;
    options.LocalUserId = ctx->user;
    options.MaxDataSizeBytes = capacity;
    socket.ApiVersion = EOS_P2P_SOCKETID_API_LATEST;
    result = EOS_P2P_ReceivePacket(ctx->p2p, &options, &remote, &socket, &channel, data, written);
    if (result == EOS_NotFound) return -1;
    if (result != EOS_Success) return (int32_t)result;
    if (channel != 0 || strcmp(socket.SocketName, ctx->socket.SocketName) != 0 ||
        (ctx->remote && remote != ctx->remote)) return -1;
    return (int32_t)EOS_ProductUserId_ToString(remote, peer, &peer_capacity);
}
int32_t tsb_eos_network_type(TsbEos* ctx) { return ctx->network_type; }
int32_t tsb_eos_retryable(int32_t result) {
    return result == (int32_t)EOS_LimitExceeded || result == (int32_t)EOS_NoConnection;
}
int32_t tsb_eos_random(char* output) {
    static const char hex[] = "0123456789abcdef";
    unsigned int value;
    int i, j;
    for (i = 0; i < 4; ++i) {
        if (rand_s(&value) != 0) return -1;
        for (j = 0; j < 8; ++j) output[i * 8 + j] = hex[(value >> (j * 4)) & 15];
    }
    output[32] = 0;
    return 0;
}
