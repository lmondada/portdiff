import type { Dispatch, SetStateAction } from "react";

import { process_event, view } from "shared/shared";
import type { Effect, Event } from "shared_types/types/shared_types";
import {
    EffectVariantRender,
    ViewModel,
    Request,
    EffectVariantLogCapability,
    LogOperationVariantError,
    LogOperationVariantInfo,
} from "shared_types/types/shared_types";
import {
    BincodeSerializer,
    BincodeDeserializer,
} from "shared_types/bincode/mod";

interface Callbacks {
    setView: Dispatch<SetStateAction<ViewModel>>;
    logInfo: (message: string) => void;
    logError: (message: string) => void;
}

export function update(
    event: Event,
    callbacks: Callbacks,
) {
    console.log("event", event);

    const serializer = new BincodeSerializer();
    event.serialize(serializer);

    const effects = process_event(serializer.getBytes());

    const requests = deserializeRequests(effects);
    for (const { id, effect } of requests) {
        processEffect(id, effect, callbacks);
    }
}

function processEffect(
    _id: number,
    effect: Effect,
    { setView, logInfo, logError }: Callbacks,
) {
    console.log("effect", effect);

    switch (effect.constructor) {
        case EffectVariantRender: {
            setView(deserializeView(view()));
            break;
        }
        case EffectVariantLogCapability: {
            const log = (effect as EffectVariantLogCapability).value;
            switch (log.constructor) {
                case LogOperationVariantError: {
                    logError((log as LogOperationVariantError).value);
                    break;
                }
                case LogOperationVariantInfo: {
                    logInfo((log as LogOperationVariantInfo).value);
                    break;
                }
            }
            break;
        }
    }
}

function deserializeRequests(bytes: Uint8Array): Request[] {
    const deserializer = new BincodeDeserializer(bytes);
    const len = deserializer.deserializeLen();
    const requests: Request[] = [];
    for (let i = 0; i < len; i++) {
        const request = Request.deserialize(deserializer);
        requests.push(request);
    }
    return requests;
}

function deserializeView(bytes: Uint8Array): ViewModel {
    return ViewModel.deserialize(new BincodeDeserializer(bytes));
}