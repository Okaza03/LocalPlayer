# Collections

[← Back to the Admin API](admin.md)

Sorted A to Z by title, a film series ends up scattered: _Batman_ under B, _The Batman_ under T, _Joker_ under J. A collection fixes that. Every title in it sits under the collection's name, next to each other, in the order the admin chose.

```
A to Z without collections        A to Z with a "Batman" collection
  Avatar                            Avatar
  Batman (1989)                     Batman (1989)        ┐
  Batman Returns (1992)             Batman Returns (1992)│ Batman, in the admin's order
  Casino                            The Batman (2022)    ┘
  The Batman (2022)                 Casino
```

There are two parts:

1. **The library** says which collection each title is in. That's all the app needs to sort; any server can do it, with or without the admin endpoints.
2. **The admin endpoints** let admins create, rename, fill, reorder and delete collections from the app's Admin page. Without them, the _Collections_ section says _Not set up_ and links here.

## 1. In the library

Two optional fields on any movie or series in [`/api/library`](library.md):

```json
{ "id": "12", "title": "The Batman", "year": 2022, "collection": "Batman", "collection_index": 2 }
```

| Field              | Type             |                                                                                   |
| ------------------ | ---------------- | --------------------------------------------------------------------------------- |
| `collection`       | string or object | The collection's name. `{"name": "Batman", …}` works too                          |
| `collection_index` | int              | Its place in the collection, `0` first. Missing: oldest first                     |

The app sorts A to Z by `collection` (or `title` when there's none), then `collection_index`, then year. Descending A to Z reverses the whole list, collection order included.

## 2. The admin endpoints

Admins only, under the [usual rules](admin.md#rules-for-every-request).

| Request                               | Body                                  | Answer                                  |
| ------------------------------------- | ------------------------------------- | --------------------------------------- |
| `GET /api/admin/collections`          |                                       | every collection                        |
| `POST /api/admin/collections`         | `{"name", "items"}`                   | `201`; `422` without a name             |
| `PATCH /api/admin/collections/<id>`   | `{"name"}` and/or `{"items"}`         | `200`; `404` if unknown                 |
| `DELETE /api/admin/collections/<id>`  |                                       | `200`. Its titles stay in the library   |

A collection:

```json
{
  "id": "3",
  "name": "Batman",
  "items": [
    { "type": "movie", "id": "m:Movies/Batman (1989)/Batman.mkv" },
    { "type": "movie", "id": "m:Movies/Batman Returns (1992)/Batman Returns.mkv" },
    { "type": "movie", "id": "m:Movies/The Batman (2022)/The Batman.mkv" }
  ]
}
```

| Field   | Type           |                                                                                                  |
| ------- | -------------- | ------------------------------------------------------------------------------------------------ |
| `id`    | string or int  | **required**, used in the URLs                                                                   |
| `name`  | string         | **required**                                                                                     |
| `items` | array          | Its titles **in order**, first shown first: `type` (`"movie"` or `"series"`) and the library `id` |

`items` in a `POST` or `PATCH` replaces the whole list, in the new order. The admin reorders by dragging, so every save sends the full list. After a change, the library must reflect it: give each title `collection` set to the name and `collection_index` set to its position in `items`.

A title belongs to one collection at a time. When a save puts a title in a collection, take it out of any other one.

Servers that can't keep an order may answer with `"movies": [ids]` and `"series": [ids]` instead of `items`; the app reads that too, but always sends `items`.

## Reference server

Collections live in `.valo/collections.json` and are applied to the library on every scan.

## Laravel example

```php
// collection_items: collection_id, item_type ('movie'|'series'), item_id, position
Route::middleware(['auth:sanctum', 'can:admin'])->prefix('admin')->group(function () {
    $show = fn (Collection $c) => ['id' => (string) $c->id, 'name' => $c->name,
        'items' => $c->items()->orderBy('position')->get()->map(fn ($i) => ['type' => $i->item_type, 'id' => (string) $i->item_id])];
    $fill = function (Collection $c, Request $r) {
        if ($r->filled('name')) $c->update(['name' => $r->name]);
        if ($r->has('items')) DB::transaction(function () use ($c, $r) {
            $items = collect($r->items);
            CollectionItem::where(fn ($q) => $items->each(fn ($i) => $q->orWhere(['item_type' => $i['type'], 'item_id' => $i['id']])))->delete();
            $c->items()->delete();
            $items->each(fn ($i, $n) => $c->items()->create(['item_type' => $i['type'], 'item_id' => $i['id'], 'position' => $n]));
        });
    };
    Route::get('/collections', fn () => Collection::all()->map($show));
    Route::post('/collections', function (Request $r) use ($fill) {
        $r->validate(['name' => 'required']);
        $fill($c = Collection::create(['name' => $r->name]), $r);
        return response()->json([], 201);
    });
    Route::patch('/collections/{collection}', function (Request $r, Collection $collection) use ($fill) {
        $fill($collection, $r);
        return [];
    });
    Route::delete('/collections/{collection}', function (Collection $collection) {
        $collection->items()->delete();
        $collection->delete();
        return [];
    });
});
```

In the library JSON, add `'collection' => $item->collectionItem?->collection->name` and `'collection_index' => $item->collectionItem?->position`.
