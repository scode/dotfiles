#!/usr/bin/env python3

# Hack to bulk-export items from 1password using op:
#
#  https://app-updates.agilebits.com/product_history/CLI
#
# At the time of this writing, tested using 0.4
#
# Usage, assuming you're logged in:
#
#   op-export.py > unencrypted-secret.json
#
# I have high hopes 1password will offer "op export" eventually :)

import logging
import json
import queue
import threading
import time
import random
import subprocess

from typing import List


def pretty_json(json_object) -> str:
    return json.dumps(json_object, indent=4, sort_keys=True)


def op_list_items() -> List[object]:
    output = subprocess.check_output("op list items",
                                     shell=True)

    return json.loads(output)


def op_get_item(uuid) -> object:
    output = subprocess.check_output(
        "op get item '{}' 2>/dev/null".format(uuid),
        shell=True,
    )

    return json.loads(output)


class GetItemThread(threading.Thread):
    def __init__(self, input_q: queue.Queue, output_q: queue.Queue) -> None:
        super().__init__()

        self.input_q = input_q
        self.output_q = output_q

    def run(self) -> None:
        try:
            while True:
                uuid = self.input_q.get(block=False)  # type: str

                n = 3
                while True:
                    n -= 1
                    try:
                        item = op_get_item(uuid)
                        break
                    except subprocess.CalledProcessError as e:
                        if n > 0:
                            # Empirically we fail a lot when attempting several requests in rapid succession, so
                            # back off a lot (and randomize for jitter).
                            logging.debug('op failed, retrying: %s', e)
                            time.sleep(5 + (random.random() * 5))
                        else:
                            raise
                self.output_q.put(item)

                self.input_q.task_done()
                logging.debug('got item %s', uuid)
        except queue.Empty:
            pass


def main() -> None:
    logging.basicConfig(level=logging.INFO)

    uuids = [item['uuid'] for item in op_list_items()]

    logging.info('total items in vault: %s', len(uuids))

    uuid_queue = queue.Queue()  # type: queue.Queue[str]
    item_queue = queue.Queue()  # type: queue.Queue[object]

    for uuid in uuids:
        uuid_queue.put(uuid)

    threads = [GetItemThread(uuid_queue, item_queue) for _ in range(8)]
    for t in threads:
        t.start()

    while not uuid_queue.empty():
        time.sleep(5)
        logging.info('still in queue: %s', uuid_queue.qsize())

    logging.info('waiting for in-flight items to complete')
    for t in threads:
        t.join()

    items = []
    while True:
        try:
            items.append(item_queue.get(block=False))
        except queue.Empty:
            break

    if len(items) != len(uuids):
        raise AssertionError('bug: unexpected number of items obtained')

    items_by_uuid = dict((item['uuid'], item) for item in items)
    for uuid in uuids:
        if uuid not in items_by_uuid:
            raise AssertionError('bug: expected item {} in item results'.format(uuid))

    print(pretty_json(items_by_uuid))


if __name__ == '__main__':
    main()
