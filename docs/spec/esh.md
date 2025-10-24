Это **фундаментальная спецификация** Исполняемого Семантического Гиперграфа (ESH), разработанная для **нативной реализации** на мультимодальной базе данных **SurrealDB**.

Этот документ определяет `Cortex` (ядро памяти) как единственный источник истины, используя `SurrealQL` и возможности `SurrealDB` для определения данных, семантики и логики исполнения.

-----

## I. Философия и Выбор SurrealDB

**Исполняемый Семантический Гиперграф (ESH)** — это не просто база данных; это *живая онтологическая модель* реальности. Все сущности (пользователи, задачи, код, воркфлоу, типы) и их n-арные отношения (связи) существуют в едином графе.

**SurrealDB** выбрана в качестве фундамента по трем причинам:

1.  **Мультимодельность:** Она нативно объединяет `документы`, `графы` и `таблицы`. Это позволяет нам хранить `HyperNode` как богатый JSON-документ (документ) и одновременно связывать их (`HyperEdge`) как первоклассных граждан графа.
2.  **Строгая Схема (Schemafull):** Позволяет нам формально определить каждый Узел и Связь, обеспечивая математическую строгость и целостность данных.
3.  **Исполняемость (Events & Functions):** Встроенные `Events` (триггеры) и `Functions` (WASM/JS) позволяют графу *реагировать* на собственные изменения, что и делает его "Исполняемым".

-----

## II. Фундаментальная Модель Данных: Два Столпа

Вся сложность Metacube сводится к двум основным "таблицам" (концепциям) в SurrealDB.

### 1\. Таблица `node`: Хранилище Сущностей (`HyperNode`)

Таблица `node` хранит **все сущности** в системе. Это и есть `HyperNode`.

  * Пользователь (`User`) — это `record<node>`.
  * Задача (`Task`) — это `record<node>`.
  * Схема (`Schema<"Task">`) — это `record<node>`.
  * **Исполняемый Воркфлоу (`Workflow`) — это `record<node>`.**
  * **И сам `HyperEdge` (n-арное отношение) — это `record<node>`.**

Это ключевое решение: сам `HyperEdge` (как концепция n-арной связи) хранится как узел, что позволяет нам моделировать *гиперграфы*.

#### Спецификация `node` (SurrealQL)

```surql
-- 1. Определяем таблицу `node` как SCHEMAFULL
-- Это гарантирует, что никакие данные, не соответствующие схеме, не могут быть записаны.
DEFINE TABLE node SCHEMAFULL;

-- 2. Определяем базовые поля для КАЖДОГО узла
-- ID (record<node>) и Timestamps (createdAt, updatedAt) встроены в SurrealDB.

-- `schema`: Это ОБЯЗАТЕЛЬНАЯ ссылка на другой узел, который описывает тип этого узла.
-- Например, для узла "Задача 123", `schema` будет `node:schema_task`.
-- Для узла "schema_task", `schema` будет `node:schema_schema`.
DEFINE FIELD schema ON node TYPE record<node>
    ASSERT $value != NONE AND $value.schema = node:schema_schema;

-- `properties`: Контейнер (JSON-объект) для фактических данных.
-- Структура этого объекта будет валидироваться схемой, на которую ссылается `schema`.
DEFINE FIELD properties ON node TYPE object;

-- `embedding`: Векторное представление узла для семантического поиска (VDB).
-- Размерность (напр., 1536 для OpenAI Ada) должна быть консистентной.
DEFINE FIELD embedding ON node TYPE array<float, 1536>;

-- `acl`: Список ссылок на узлы "Правил Доступа" (Access Control List).
DEFINE FIELD acl ON node TYPE array<record<node>>;
```

### 2\. Таблица `relates`: Связующее Ребро (`Binary Edge`)

Если `HyperEdge` (n-арная связь) сам является `node`, как нам связать с ним участников? Через стандартное *бинарное ребро* SurrealDB.

Мы называем это ребро `relates` (относится). Оно является "клеем" гиперграфа.

**Сценарий: Моделирование `HyperEdge`**
Мы хотим смоделировать: "Алиса (`User`) назначила Задачу 123 (`Task`) Бобу (`User`)".

1.  Мы создаем `node` для этого `HyperEdge`:
    `CREATE node:assignment_456 SET schema = node:schema_assignment, properties = { ... };`
2.  Мы связываем участников с этим узлом- ребром `relates`:
    `RELATE (node:user_alice) -> relates { role: 'assigner' } -> (node:assignment_456);`
    `RELATE (node:task_123) -> relates { role: 'task' } -> (node:assignment_456);`
    `RELATE (node:user_bob) -> relates { role: 'assignee' } -> (node:assignment_456);`

#### Спецификация `relates` (SurrealQL)

```surql
-- 1. Определяем таблицу `relates` как SCHEMAFULL
DEFINE TABLE relates SCHEMAFULL;

-- 2. `in` и `out` (встроенные) ссылаются на `record<node>`
-- (SurrealDB обрабатывает это автоматически)

-- 3. `role`: Семантическая роль, которую `in` узел играет в `out` узле-гиперребре.
-- Это СЕРДЦЕ семантики гиперграфа.
DEFINE FIELD role ON relates TYPE string
    ASSERT $value != NONE;

-- 4. `properties`: Дополнительные свойства самой связи (напр., 'weight', 'priority')
DEFINE FIELD properties ON relates TYPE object;
```

-----

## III. Семантика (The "S" in ESH): `SCHEMAFULL`

Семантика обеспечивается через **валидацию**. Мы используем `node:schema_...` для определения структуры `node.properties`.

#### Пример: Определение Схемы Задачи (SurrealQL)

```surql
-- 1. Создаем узел для самой схемы
CREATE node:schema_task SET
    schema = node:schema_schema,
    properties = {
        name: "Task",
        -- Валидация полей
        fields: [
            { name: "title", type: "string", required: true },
            { name: "status", type: "string", default: "todo" },
            { name: "dueDate", type: "datetime", required: false }
        ]
    };

-- 2. (ВНЕ SURREALDB) - Логика Cortex/Axon
-- Валидатор в Cortex (Rust) теперь знает, что при
-- CREATE node SET schema = node:schema_task
-- он должен проверить `properties` на соответствие `node:schema_task.properties.fields`

-- 3. (АЛЬТЕРНАТИВА) - Использование встроенных ASSERTIONS
-- Если схема простая, можно использовать ASSERT
DEFINE FIELD properties.title ON node WHERE schema = node:schema_task TYPE string ASSERT $value != NONE;
DEFINE FIELD properties.status ON node WHERE schema = node:schema_task TYPE string DEFAULT "todo";
```

*Инженерное решение:* Использование `ASSERT` (Вариант 3) жестко привязывает логику к БД. Использование `schema` (Вариант 2) и валидация на уровне `Cortex` (Rust) более гибкое, что мы и выбираем.

-----

## IV. Исполняемость (The "E" in ESH): `EVENTS` и `FUNCTIONS`

Это то, что делает граф "живым".

### 4.1. `EVENTS`: Реактивность (Cortex -\> Axon)

`EVENTS` в SurrealDB позволяют базе данных *реагировать* на изменения данных и *уведомлять* `Axon` (логический слой).

```surql
-- Спецификация: Уведомить Axon о смене статуса задачи
DEFINE EVENT on_task_status_change ON node
-- Триггер: только для узлов типа "Task" И когда "status" ИЗМЕНИЛСЯ
WHEN
    $before.schema = node:schema_task AND
    $after.schema = node:schema_task AND
    $before.properties.status != $after.properties.status
-- Действие: Отправить HTTP POST вебхук на endpoint Axon
THEN (
    LET $body = {
        nodeId: $after.id,
        oldStatus: $before.properties.status,
        newStatus: $after.properties.status
    };
    -- Axon получит это и запустит нужный Workflow
    http::post("http://axon-service:8080/v1/webhook/task_changed", $body)
);
```

### 4.2. `FUNCTIONS`: Инструменты (Cortex Toolbox)

`FUNCTIONS` позволяют определить кастомную логику (Toolbox) прямо в `Cortex`. Агенты `Axon` могут вызывать их через `SurrealQL`.

```surql
-- 1. Определяем функцию (Tool) для запуска теста (например, в JS)
DEFINE FUNCTION fn::code::run_test($repo_url: string, $test_name: string) {
    -- ... логика выполнения теста (может быть JS или Rust/WASM)
    -- ... (в реальности, это может быть HTTP-запрос к CI/CD)
    RETURN { status: "running", jobId: "xyz-123" };
};

-- 2. Агент Axon вызывает этот инструмент
-- Axon отправляет этот запрос в Cortex (SurrealDB)
LET $test_result = fn::code::run_test("http://github.com/...", "test_auth");
RETURN $test_result;
```

-----

## V. Запросы: Жизнь в Гиперграфе (Примеры SurrealQL)

Этот дизайн позволяет выполнять невероятно мощные семантические запросы.

### Запрос 1: "Найти все задачи, назначенные Бобу"

```surql
-- 1. Найти узел Боба
LET $bob = node:user_bob;

-- 2. Найти все "назначения" (узлы-гиперребра), где Боб - 'assignee'
LET $assignments = SELECT out FROM relates WHERE in = $bob AND role = 'assignee';

-- 3. Из этих назначений, найти все узлы, играющие роль 'task'
SELECT in FROM relates
WHERE
    out IN $assignments AND
    role = 'task' AND
    -- Дополнительно убедимся, что это задачи
    in.schema = node:schema_task
FETCH in; -- "FETCH" загружает полные данные узлов-задач
```

### Запрос 2: "Найти все комментарии к Задаче 123" (Модель Вложенности)

Мы можем смоделировать это как `HyperEdge` "Содержит" (`Contains`).
`RELATE (node:task_123) -> relates { role: 'parent' } -> (node:hyperedge_contains_1);`
`RELATE (node:comment_abc) -> relates { role: 'child' } -> (node:hyperedge_contains_1);`

```surql
-- Найти всех 'child' для 'parent' = Task 123
LET $task = node:task_123;
LET $containment_edges = SELECT out FROM relates WHERE in = $task AND role = 'parent';

SELECT in FROM relates
WHERE
    out IN $containment_edges AND
    role = 'child'
FETCH in;
```

*Оптимизация:* SurrealDB также позволяет `(parent)->(contains_edge)->(child)`. Если отношение *всегда* бинарное (как "комментарий к посту"), мы можем использовать простое бинарное ребро `comments_on` вместо `relates`, что еще быстрее.

-----

## VI. Сводка Спецификации

| Компонент | Сущность SurrealDB | Назначение |
| :--- | :--- | :--- |
| **`HyperNode`** | `TABLE node` (Record) | Любая Сущность (Задача, Пользователь, Схема) |
| **`HyperEdge` (n-арный)** | `TABLE node` (Record) | Узел, представляющий n-арную связь (напр., "Назначение") |
| **Связь Участника** | `TABLE relates` (Edge) | **Бинарный** клей. Связывает `node` с `node` (гиперребром) через `role`. |
| **Схема / Тип** | `node.schema` (Link) | Ссылка на `node` типа `schema_schema`, определяющий структуру. |
| **Валидация** | `SCHEMAFULL` / `ASSERT` | Обеспечение строгой целостности данных. |
| **Исполняемость** | `EVENTS` | Реакция на изменения (триггеры для `Axon`). |
| **Инструменты** | `FUNCTIONS` | Встроенная логика (Toolbox), вызываемая `Axon`. |
| **Семантика (Поиск)** | `node.embedding` (Array) | Хранилище векторов (синхронизируется с `Qdrant` для ANN-поиска). |