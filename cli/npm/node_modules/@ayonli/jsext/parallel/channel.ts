import { Channel } from "../chan.ts";
import { ChannelMessage } from "./types.ts";
import { id, isMainThread } from "../env.ts";

const channelStore = new Map<number, {
    channel: Channel<any>,
    raw: Pick<Channel<any>, "send" | "close">;
    writers: Array<(type: "send" | "close", msg: any, channelId: number) => void>,
    counter: number;
}>();

export function isChannelMessage(msg: any): msg is ChannelMessage {
    return msg
        && typeof msg === "object"
        && ["send", "close"].includes(msg.type)
        && typeof msg.channelId === "number";
}

export async function handleChannelMessage(msg: ChannelMessage) {
    const record = channelStore.get(msg.channelId);

    if (!record)
        return;

    if (msg.type === "send") {
        await record.raw.send(msg.value);
    } else if (msg.type === "close") {
        const { value: err, channelId } = msg;
        record.raw.close(err);
        channelStore.delete(channelId);

        if (isMainThread && record.writers.length > 1) {
            // distribute the channel close event to all threads
            record.writers.forEach(write => {
                write("close", err, channelId);
            });
        }
    }
}

function wireChannel(
    channel: Channel<any>,
    channelWrite: (type: "send" | "close", msg: any, channelId: number) => void
) {
    const channelId = channel[id] as number;

    if (!channelStore.has(channelId)) {
        const send = channel.send.bind(channel);
        const close = channel.close.bind(channel);

        channelStore.set(channelId, {
            channel,
            raw: { send, close },
            writers: [channelWrite],
            counter: 0,
        });

        Object.defineProperties(channel, {
            send: {
                configurable: true,
                writable: true,
                value: async (data: any) => {
                    const record = channelStore.get(channelId);

                    if (record) {
                        const channel = record.channel;

                        if (channel["state"] !== 1) {
                            throw new Error("the channel is closed");
                        }

                        const write = record.writers[record.counter++ % record.writers.length];
                        await Promise.resolve(write!("send", data, channelId));
                    }
                },
            },
            close: {
                configurable: true,
                writable: true,
                value: (err: Error | null = null) => {
                    const record = channelStore.get(channelId);

                    if (record) {
                        channelStore.delete(channelId);
                        const channel = record.channel;

                        record.writers.forEach(write => {
                            write("close", err, channelId);
                        });

                        // recover to the original methods
                        Object.defineProperties(channel, {
                            send: {
                                configurable: true,
                                writable: true,
                                value: record.raw.send,
                            },
                            close: {
                                configurable: true,
                                writable: true,
                                value: record.raw.close,
                            },
                        });

                        channel.close(err);
                    }
                },
            },
        });
    } else {
        const record = channelStore.get(channelId);
        record!.writers.push(channelWrite);
    }
}

export function wrapChannel(
    channel: Channel<any>,
    channelWrite: (type: "send" | "close", msg: any, channelId: number) => void
): object {
    wireChannel(channel, channelWrite);
    return { "@@type": "Channel", "@@id": channel[id], "capacity": channel.capacity };
}

export function unwrapChannel(
    obj: { "@@type": "Channel", "@@id": number; capacity?: number; },
    channelWrite: (type: "send" | "close", msg: any, channelId: number) => void
): Channel<any> {
    const channelId = obj["@@id"];
    let channel = channelStore.get(channelId)?.channel;

    if (!channel) {
        channel = Object.assign(Object.create(Channel.prototype), {
            [id]: channelId,
            capacity: obj.capacity ?? 0,
            buffer: [],
            producers: [],
            consumers: [],
            error: null,
            state: 1,
        });
    }

    wireChannel(channel as Channel<any>, channelWrite);

    return channel as Channel<any>;
}

