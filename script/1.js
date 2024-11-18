// ==UserScript==
// @name         bilibili视频屏蔽
// @namespace    http://tampermonkey.net/
// @version      2024-02-07
// @description  bilibili视频屏蔽脚本
// @author       AntonTwelve
// @match        https://space.bilibili.com/*
// @match        https://www.bilibili.com/*
// @grant        GM_addStyle
// @grant        GM_xmlhttpRequest
// ==/UserScript==

(function () {
    "use strict";
    // Your code here...
    var server_host = "http://127.0.0.1:22332";
    // BV号对应的mid缓存, 给热门页屏蔽用
    var bv2mid_cache = new Map();
    var blocker = null;

    //添加样式
    GM_addStyle(`
     .alert_wrapper {
         z-index: 9999;
         position: fixed;
         top: calc(100vh - 58px);
         left: 20px;
         height: 38px;
         width: 280px;
         background-color: #fef0f0;
         border: 1px solid #fde2e2;
         border-radius: 4px;
         overflow: hidden;
         display: flex;
         align-items: center;
         justify-content: center;
     }
 
     .alert_wrapper_init_state {
         opacity: 0;
         transform: translateX(-50%);
         transition: opacity .3s,transform .4s;
     }
 
     .alert_wrapper_fade_in {
         opacity: 1;
         transform: translateX(0);
         transition: opacity .3s,transform .4s;
     }
 
     .alert_text {
         color: #f56c6c;
     }

     .block_context_menu{
        z-index: 999;
        position: fixed;
        user-select: none;
        background-color: #dc2626;
        color: white;
        height: 30px;
        width: 76px;
        text-align: center;
        font-size: 14px;
        font-weight: 500;
        border-radius: 4px;
        line-height: 30px;
     }`)

    /**
     * 用户空间显示屏蔽按钮
     */
    function user_space_block() {
        var is_exist = false;

        function wait_for_element() {
            let btn_ele = document.querySelector(".h>.wrapper>.h-inner>.h-action");
            if (btn_ele) {
                add_block_btn(btn_ele);
                check_is_user_blocked();
                return;
            }
            setTimeout(wait_for_element, 500);  //500ms检测一次
        }

        function set_style() {
            GM_addStyle(`
                #t_block_btn {
                    background-color: #dc2626;
                    color: white;
                    height: 30px;
                    width: 76px;
                    text-align: center;
                    font-size: 14px;
                    display: inline-block;
                    margin: 0 20px 17px 0;
                    box-shadow: 0 0 0 2px hsla(0,0%,100%,.3);
                    border: 0;
                    font-weight: 500;
                    border-radius: 4px;
                    line-height: 30px;
                }
                #t_block_btn:hover {
                    background-color: #df3b3b;
                }`)
        }
        set_style();

        function add_block_btn(ele) {
            let new_button = document.createElement('button');
            new_button.textContent = '屏蔽用户';
            new_button.setAttribute("id", "t_block_btn");
            new_button.style.display = "none";  //先设置为不可见状态, 服务端返回数据后再显示
            ele.querySelector(".h-add-to-black").style.float = "right";
            ele.insertBefore(new_button, ele.querySelector(".be-dropdown"));
            new_button.addEventListener('click', function () {
                if (!is_exist) {
                    send_block_info();
                }
                else {
                    send_remove_req();
                }
            });
        }

        function get_user_name() {
            let name_ele = document.getElementById("h-name");
            if (!name_ele) return null;
            return name_ele.textContent;
        }

        function get_user_mid() {
            let current_url = window.location.href.substring(27);
            let index_1 = current_url.indexOf('?');
            let index_2 = current_url.indexOf('/');
            if (index_1 == -1 && index_2 == -1) return current_url;
            if (index_1 == -1) index_1 = 9999;
            if (index_2 == -1) index_2 = 9999;
            return current_url.substring(0, Math.min(index_1, index_2));
        }

        function send_block_info() {
            let user_name = get_user_name();
            let user_mid = get_user_mid();
            if (!user_mid || !user_name) return;
            let data = new FormData();
            data.append("mid", user_mid);
            data.append("username", user_name);
            GM_xmlhttpRequest({
                method: "POST",
                url: server_host + "/block",
                data: data,
                onload: function (response) {
                    if (response.responseText === "OK") {
                        user_blocked();
                    }
                    else if (response.responseText === "ERR1") {
                        b_blocker_alert("错误的mid");
                    }
                    else if (response.responseText === "ERR2") {
                        b_blocker_alert("插入数据库错误")
                    }
                    else {
                        b_blocker_alert("错误")
                    }
                },
                onerror: function (response) {
                    b_blocker_alert("error!");
                }
            });
        }

        function send_remove_req() {
            let user_mid = get_user_mid();
            if (!user_mid) return;
            let data = new FormData();
            data.append("mid", user_mid);
            GM_xmlhttpRequest({
                method: "POST",
                url: server_host + "/remove",
                data: data,
                onload: function (response) {
                    if (response.responseText == "OK") {
                        user_unblocked();
                    }
                    else if (response.responseText == "ERR1") b_blocker_alert("错误的mid")
                    else if (response.responseText == "ERR2") b_blocker_alert("解除屏蔽失败")
                }
            })
        }

        function check_is_user_blocked() {
            let user_mid = get_user_mid();
            if (!user_mid) return;
            GM_xmlhttpRequest({
                method: "GET",
                url: server_host + "/isExist?mid=" + user_mid,
                onload: function (response) {
                    if (response.responseText === "True") {
                        user_blocked();
                    }
                    // 服务端在线, 可以显示按钮
                    let btn = document.getElementById("t_block_btn");
                    btn.style.display = "inline-block";
                }
            });
        }

        function user_blocked() {
            let btn = document.getElementById("t_block_btn");
            btn.textContent = "已屏蔽";
            is_exist = true;
        }

        function user_unblocked() {
            let btn = document.getElementById("t_block_btn");
            btn.textContent = "屏蔽用户";
            is_exist = false;
        }

        wait_for_element();
    }

    function b_blocker_alert(text) {
        let ele = document.createElement("div");
        ele.setAttribute("class", "alert_wrapper alert_wrapper_init_state");
        let text_span = document.createElement("span");
        text_span.setAttribute("class", "alert_text");
        text_span.innerText = "屏蔽脚本:" + text;
        ele.appendChild(text_span);
        document.body.appendChild(ele);
        ele.offsetWidth;    //触发重绘
        ele.classList.remove("alert_wrapper_init_state");
        ele.classList.add("alert_wrapper_fade_in");

        setTimeout(() => {
            ele.classList.remove("alert_wrapper_fade_in");
            ele.classList.add("alert_wrapper_init_state");
            ele.addEventListener("transitionend", () => {
                ele.remove();
            })
        }, 1500);
    }

    /**
     * 移除右键屏蔽菜单
     */
    function remove_context_menu() {
        let eles = document.querySelectorAll(".block_context_menu");
        eles.forEach((ele) => {
            ele.remove();
        })
    }

    /**
     * 弹出右键屏蔽菜单
     * @param {*} x 
     * @param {*} y 
     * @param {*} callback 
     */
    function pop_right_click_block_btn(x, y, mid, name) {
        let menu_div = document.createElement('div');
        menu_div.setAttribute("class", "block_context_menu");
        menu_div.style.left = `${x}px`;
        menu_div.style.top = `${y}px`;
        menu_div.innerText = "屏蔽用户";
        document.body.appendChild(menu_div);
        window.addEventListener("click", remove_context_menu);
        menu_div.addEventListener("click", () => {
            block_user(mid, name);
        })
    }

    /**
     * 屏蔽某个用户
     * @param {*} user_mid 
     * @param {*} user_name 
     */
    function block_user(user_mid, user_name) {
        if (!user_mid || !user_name) return;
        let data = new FormData();
        data.append("mid", user_mid);
        data.append("username", user_name);
        GM_xmlhttpRequest({
            method: "POST",
            url: server_host + "/block",
            data: data,
            onload: function (response) {
                if (response.responseText === "ERR1") {
                    b_blocker_alert("错误的mid");
                }
                else if (response.responseText === "ERR2") {
                    b_blocker_alert("插入数据库错误")
                }
                else {
                    if (blocker !== null) {
                        setTimeout(()=>{
                            blocker.find_and_block();
                        }, 100);
                    }
                }
            },
            onerror: function (response) {
                b_blocker_alert("error!");
            }
        });
    }

    class video_card_blocker {
        constructor() {
            this.is_server_online = true;
            this.query_cache = new Map();
            this.video_card_selector = ".bili-video-card";
            this.cache_update = true;
            this.cache_update_time = 3000;
            this.block_loop_time = 500;
        }

        block_video_card(ele) {
            ele.style.display = "none";
        }

        is_user_blocked(card, user_mid, self) {
            let cache_ret = this.query_cache.get(user_mid);
            if (cache_ret === true) {
                this.block_video_card(card);
                return;
            }
            else if (cache_ret === false) return;
            GM_xmlhttpRequest({
                method: "GET",
                url: server_host + "/isExist?mid=" + user_mid,
                onload: function (response) {
                    if (response.responseText === "True") {
                        self.block_video_card(card);
                        self.query_cache.set(user_mid, true);
                    }
                    else if (response.responseText === "False") {
                        self.query_cache.set(user_mid, false);
                    }
                },
                onerror: function (response) {
                    //只有第一次才弹出提示
                    if (self.is_server_online)
                        b_blocker_alert("服务端不在线");
                    self.is_server_online = false;
                }
            });
        }

        right_click_handler(e) {
            e.preventDefault();
            let target = e.target;
            //找之前插入的block_script_id属性, 最多找10层
            for (let i = 0; i < 10; i++) {
                let block_id = target.getAttribute("block_script_id");
                let block_name = target.getAttribute("block_script_name");
                if (block_id != null && block_name != null) {
                    remove_context_menu();
                    pop_right_click_block_btn(e.clientX, e.clientY, block_id, block_name);
                    return;
                }
                target = target.parentNode;
            }
            b_blocker_alert("未找到ID, 请稍后再试");
        }

        get_user_name_from_video_card(ele) {
            let name_ele = ele.querySelector(".bili-video-card__wrap>.bili-video-card__info>.bili-video-card__info--right>.bili-video-card__info--bottom>.bili-video-card__info--owner>.bili-video-card__info--author");
            let name = name_ele.innerText;
            return name;
        }

        get_mid_from_video_card(ele) {
            let a_ele = ele.querySelector(".bili-video-card__wrap>.bili-video-card__info>.bili-video-card__info--right>.bili-video-card__info--bottom>a");
            if (!a_ele) return null;
            let url = a_ele.getAttribute("href");
            let match_result = url.match(/\/\/space\.bilibili\.com\/([0-9]+)/);
            if (match_result == null) return null;
            return match_result[1];
        }

        find_and_block() {
            if (!this.is_server_online) return;
            let bili_video_cards = document.querySelectorAll(this.video_card_selector);
            for (let i = 0; i < bili_video_cards.length; i++) {
                let mid = this.get_mid_from_video_card(bili_video_cards[i]);
                if (!mid) continue;
                let name = this.get_user_name_from_video_card(bili_video_cards[i]);
                bili_video_cards[i].setAttribute("block_script_id", mid);
                bili_video_cards[i].setAttribute("block_script_name", name);
                bili_video_cards[i].addEventListener("contextmenu", this.right_click_handler);
                this.is_user_blocked(bili_video_cards[i], mid, this);
            }

            setTimeout(() => {
                this.find_and_block();
            }, this.block_loop_time);
        }

        run() {
            this.find_and_block();
            if (this.cache_update)
                setInterval(() => {
                    //n ms清空一次缓存
                    this.query_cache.clear();
                }, this.cache_update_time);
        }
    }

    /**
     * 主页屏蔽
     */
    function home_page_block() {
        blocker = new video_card_blocker();
        blocker.run();
    }

    /**
     * 热门页屏蔽
     */
    function popular_page_block() {
        class popular_page_video_blocker extends video_card_blocker {
            constructor() {
                super();
                this.video_card_selector = ".video-card";
                // this.cache_update_time = 3000;
                this.block_loop_time = 10000;
            }

            // is_video_bv_blocked
            is_user_blocked(card, video_bv, self) {
                let cache_ret = this.query_cache.get(video_bv);
                if (cache_ret === true) {
                    this.block_video_card(card);
                    return;
                }
                else if (cache_ret === false) return;
                GM_xmlhttpRequest({
                    method: "GET",
                    url: server_host + "/blockBV?bv=" + video_bv,
                    onload: function (response) {
                        let ret_data = JSON.parse(response.responseText);
                        if (ret_data["msg"] !== "OK") {
                            return;
                        }
                        bv2mid_cache.set(video_bv, ret_data["mid"]);
                        if (ret_data["result"] === "True") {
                            self.block_video_card(card);
                            self.query_cache.set(video_bv, true);
                        }
                        else if (ret_data["result"] === "False") {
                            self.query_cache.set(video_bv, false);
                        }
                    },
                    onerror: function (response) {
                        if (self.is_server_online)
                            b_blocker_alert("服务端不在线");
                        self.is_server_online = false;
                    }
                });
            }

            right_click_handler(e) {
                e.preventDefault();
                let target = e.target;
                //找之前插入的block_script_id属性, 最多找10层
                for (let i = 0; i < 10; i++) {
                    let block_id = target.getAttribute("block_script_id");
                    block_id = bv2mid_cache.get(block_id);
                    let block_name = target.getAttribute("block_script_name");
                    if (block_id != null && block_name != null) {
                        remove_context_menu();
                        pop_right_click_block_btn(e.clientX, e.clientY, block_id, block_name);
                        return;
                    }
                    target = target.parentNode;
                }
                b_blocker_alert("未找到ID, 请稍后再试");
            }

            get_user_name_from_video_card(ele) {
                let name_ele = ele.querySelector(".video-card__info>div>.up-name>.up-name__text");
                let name = name_ele.innerText;
                return name;
            }

            // get_bv_from_video_card
            get_mid_from_video_card(ele) {
                let a_ele = ele.querySelector(".video-card__content>a");
                if (!a_ele) return null;
                let url = a_ele.getAttribute("href");
                let match_result = url.match(/\/\/www\.bilibili\.com\/video\/(BV[A-Za-z0-9]+)/);
                if (match_result == null) return null;
                return match_result[1];
            }
        }
        blocker = new popular_page_video_blocker();
        blocker.run();
        setTimeout(()=>{
            blocker.find_and_block();
        }, 1000);
    }

    /**
     * 视频页屏蔽
     */
    function video_page_block() {
        class video_page_video_blocker extends video_card_blocker {
            constructor() {
                super();
                this.video_card_selector = ".video-page-card-small,.video-page-operator-card-small";
            }

            /**
             * 从video card中找到用户名称并返回
             * @param {*} ele 
             * @returns 
             */
            get_user_name_from_video_card(ele) {
                let name_ele = ele.querySelector(".card-box>.info>.upname>a>.name");
                let name = name_ele.innerText;
                return name;
            }

            /**
             * 从video card中找到用户ID返回
             * @param {*} ele video card元素
             * @returns 用户ID或null
             */
            get_mid_from_video_card(ele) {
                let a_ele = ele.querySelector(".card-box>.info>.upname>a");
                if (!a_ele) return null;
                let url = a_ele.getAttribute("href");
                let mid = url.substring(21, url.length - 1);
                if (!isNaN(mid)) return mid;
                return null;
            }
        }
        blocker = new video_page_video_blocker();
        blocker.run();
    }

    var url = window.location.href;
    if (url.startsWith("https://space.bilibili.com/")) user_space_block();
    else if (url.startsWith("https://www.bilibili.com/video/BV") || url.startsWith("https://www.bilibili.com/video/av")) video_page_block();
    else if (url.startsWith("https://www.bilibili.com/v/popular/all/")) popular_page_block();
    else if (url === "https://www.bilibili.com/" || url.startsWith("https://www.bilibili.com/?")) home_page_block();
})();
